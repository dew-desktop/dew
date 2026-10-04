--!strict
--[[
	System Monitor: what the machine is doing, in the desktop clock's colours.

	    cargo run -- examples/system/monitor
	    cargo run -- snapshot examples/system/monitor --after 6 -o monitor.png

	From the top:

	  HEADER      a heartbeat that flips on every sample, the OS, the battery
	              when there is one, and uptime.
	  CPU         total load, a minute of history whose older bars fade, and a
	              heat tile per core laid out by one UIGridLayout (past sixteen
	              threads, each tile averages a run of them).
	  MEMORY      used and free as two segments whose widths are their
	              UIFlexItem GrowRatios -- the bytes themselves, no arithmetic.
	  NETWORK     download rising and upload hanging from one centre line, both
	              against the same scale, so the two can be compared at a glance.
	  STORAGE     every fixed drive, fullest in warm colours.
	  PROCESSES   the five busiest by name; click CPU / MEM to re-sort. The
	              choice is kept in `desktop.Storage`, like the clock's 12h/24h.

	NOTHING IS POSITIONED BY HAND. Rows are UIListLayouts, the spare width in a
	row goes to whichever child has a UIFlexItem, the graphs are lists with
	HorizontalFlex = Fill so sixty bars share any width evenly, and the cores are
	a grid. The only numbers below are sizes.

	NOTHING REDRAWS PER FRAME. Every update happens in `System.OnSample`, once a
	second, so the monitor costs one repaint a second and does not appear in its
	own graph.
]]

local widget = assert(desktop.Widget, "dew.toml must grant `widget`")
local System = assert(desktop.System, "dew.toml must grant `system`")

local WIDTH = 340
local PAD = 14 -- card inset
local GAP = 10 -- between sections
local INSET = 12 -- panel inset
local ROW_GAP = 8 -- between rows in a panel
local INNER = WIDTH - 2 * PAD - 2 * INSET -- a panel row's width

-- The clock's palette: sky blue to periwinkle, white type with a dark edge.
local SKY = Color3.fromRGB(72, 148, 226)
local LILAC = Color3.fromRGB(132, 120, 218)
local RIM = Color3.fromRGB(210, 232, 255)
local INK = Color3.fromRGB(30, 48, 80)
local WHITE = Color3.fromRGB(255, 255, 255)

-- Load colours, cool to hot. Cool is nearly white so a quiet machine reads as
-- calm on the gradient; warm and hot are reserved for something worth seeing.
local COOL = Color3.fromRGB(196, 250, 236)
local WARM = Color3.fromRGB(255, 212, 120)
local HOT = Color3.fromRGB(255, 118, 140)
local DOWN = Color3.fromRGB(150, 245, 215)
local UP = Color3.fromRGB(255, 196, 228)

local function face(family: string, weight: Enum.FontWeight?): Font
	return Font.new(`rbxasset://fonts/families/{family}.json`, weight or Enum.FontWeight.Regular)
end
local DISPLAY = face("Montserrat", Enum.FontWeight.Heavy)
local TITLE = face("Montserrat", Enum.FontWeight.Bold)
local CAPS = face("Montserrat", Enum.FontWeight.Bold)
local BODY = face("SourceSansPro", Enum.FontWeight.SemiBold)
local MONO = face("RobotoMono")

-- Formatting -----------------------------------------------------------------

local function bytes(n: number): string
	local units = { "B", "KB", "MB", "GB", "TB" }
	local i = 1
	while n >= 1024 and i < #units do
		n /= 1024
		i += 1
	end
	if i == 1 then
		return `{math.floor(n)} B`
	end
	return string.format(if n >= 100 then "%.0f %s" else "%.1f %s", n, units[i])
end

local function rate(n: number): string
	return bytes(n) .. "/s"
end

local function duration(seconds: number): string
	local d = seconds // 86400
	local h = (seconds % 86400) // 3600
	local m = (seconds % 3600) // 60
	if d > 0 then
		return `{d}d {h}h`
	elseif h > 0 then
		return `{h}h {m}m`
	end
	return `{m}m`
end

local function mix(a: Color3, b: Color3, t: number): Color3
	t = math.clamp(t, 0, 1)
	return Color3.new(a.R + (b.R - a.R) * t, a.G + (b.G - a.G) * t, a.B + (b.B - a.B) * t)
end

local function heat(load: number): Color3
	if load < 55 then
		return COOL
	elseif load < 80 then
		return mix(COOL, WARM, (load - 55) / 25)
	end
	return mix(WARM, HOT, (load - 80) / 20)
end

--- The drive's own name where it has one worth showing: `C:` rather than `C:\`.
local function driveName(mount: string): string
	local letter = string.match(mount, "^(%a:)[\\/]?$")
	return letter or mount
end

-- Building -------------------------------------------------------------------

--- An instance of `class` with `props`, parented last so it arrives whole.
local function make(class: string, props: { [string]: any }): any
	local instance = Instance.new(class)
	local parent = props.Parent
	for key, value in props do
		if key ~= "Parent" then
			(instance :: any)[key] = value
		end
	end
	instance.Parent = parent
	return instance
end

local function rounded(parent: Instance, radius: number)
	make("UICorner", { CornerRadius = UDim.new(0, radius), Parent = parent })
end

local function inset(parent: Instance, x: number, y: number)
	make("UIPadding", {
		PaddingLeft = UDim.new(0, x),
		PaddingRight = UDim.new(0, x),
		PaddingTop = UDim.new(0, y),
		PaddingBottom = UDim.new(0, y),
		Parent = parent,
	})
end

--- A UIListLayout in LayoutOrder. SortOrder defaults to Name on every layout,
--- which is rarely what the LayoutOrder beside it meant.
local function list(parent: Instance, direction: Enum.FillDirection, gap: number, props: { [string]: any }?): UIListLayout
	local layout = make("UIListLayout", {
		FillDirection = direction,
		SortOrder = Enum.SortOrder.LayoutOrder,
		Padding = UDim.new(0, gap),
		Parent = parent,
	})
	for key, value in props or {} do
		(layout :: any)[key] = value
	end
	return layout
end

--- Take the row's spare width. The one line that makes a spacer, a stretching
--- label or a bar that fills what its neighbours leave.
local function fill(instance: Instance)
	make("UIFlexItem", { FlexMode = Enum.UIFlexMode.Fill, Parent = instance })
end

local function blank(name: string, order: number, size: UDim2, parent: Instance): Frame
	return make("Frame", {
		Name = name,
		LayoutOrder = order,
		Size = size,
		BackgroundTransparency = 1,
		Parent = parent,
	})
end

local function label(parent: Instance, props: { [string]: any }): TextLabel
	local text = make("TextLabel", {
		BackgroundTransparency = 1,
		TextColor3 = WHITE,
		FontFace = BODY,
		TextSize = 13,
		TextXAlignment = Enum.TextXAlignment.Left,
		TextYAlignment = Enum.TextYAlignment.Center,
		Parent = parent,
	})
	for key, value in props do
		(text :: any)[key] = value
	end
	return text
end

--- A small capitalised heading, the panel's name.
local function caption(parent: Instance, text: string, order: number): TextLabel
	return label(parent, {
		Name = "Caption",
		LayoutOrder = order,
		Text = text,
		FontFace = CAPS,
		TextSize = 10,
		TextTransparency = 0.2,
		AutomaticSize = Enum.AutomaticSize.X,
		Size = UDim2.new(0, 0, 1, 0),
	})
end

--- The dark edge the clock gives its digits, for the large numerals.
local function edged(text: TextLabel)
	make("UIStroke", { Thickness = 1, Color = INK, Transparency = 0.55, Parent = text })
end

--- A frosted panel holding rows. Each row is asked for at its own height, and
--- the panel's own height follows, so the window can be sized to the content
--- before it exists. A row built hidden takes no height, and the list skips it.
type Panel = {
	frame: Frame,
	height: number,
	row: (self: Panel, name: string, height: number, shown: boolean?) -> Frame,
}

local function panel(parent: Instance, name: string, order: number): Panel
	local frame = make("Frame", {
		Name = name,
		LayoutOrder = order,
		Size = UDim2.new(1, 0, 0, 0),
		BackgroundColor3 = WHITE,
		BackgroundTransparency = 0.87,
		Parent = parent,
	})
	rounded(frame, 10)
	make("UIStroke", { Thickness = 1, Color = WHITE, Transparency = 0.78, Parent = frame })
	inset(frame, INSET, INSET)
	list(frame, Enum.FillDirection.Vertical, ROW_GAP)

	local self = { frame = frame, height = 2 * INSET - ROW_GAP } :: any
	local rows = 0
	function self.row(this: Panel, rowName: string, height: number, shown: boolean?): Frame
		rows += 1
		local row = blank(rowName, rows, UDim2.new(1, 0, 0, height), frame)
		if shown == false then
			row.Visible = false
		else
			this.height += height + ROW_GAP
			frame.Size = UDim2.new(1, 0, 0, this.height)
		end
		return row
	end
	return self
end

--- A row of `count` bars sharing the row's width evenly, standing on
--- `align`'s edge. HorizontalFlex = Fill is what shares the width: every bar
--- asks for 1px and the layout hands each the same part of what is left.
local function bars(parent: Instance, count: number, align: Enum.VerticalAlignment, colour: Color3): { Frame }
	list(parent, Enum.FillDirection.Horizontal, 2, {
		HorizontalFlex = Enum.UIFlexAlignment.Fill,
		VerticalAlignment = align,
	})
	local made = {}
	for i = 1, count do
		local bar = make("Frame", {
			Name = string.format("Bar%02d", i),
			LayoutOrder = i,
			Size = UDim2.new(0, 1, 0, 0),
			BackgroundColor3 = colour,
			BackgroundTransparency = 1,
			Parent = parent,
		})
		rounded(bar, 2)
		made[i] = bar
	end
	return made
end

--- Set the bars in `row` from `series`, newest on the right. Missing samples are hidden
--- rather than drawn as zero: a minute that has not happened yet is not idle.
--- Older bars fade, so the graph reads as moving even between samples.
local function plot(row: { Frame }, series: { number }, scale: number, tint: ((number) -> Color3)?)
	local count = #row
	local offset = count - #series
	for i, bar in row do
		local value = series[i - offset]
		if value == nil then
			bar.BackgroundTransparency = 1
		else
			local ratio = math.clamp(value / scale, 0, 1)
			bar.Size = UDim2.new(0, 1, ratio, 2)
			bar.BackgroundTransparency = 0.05 + 0.6 * (1 - i / count)
			if tint then
				bar.BackgroundColor3 = tint(value)
			end
		end
	end
end

-- The card ------------------------------------------------------------------

local card = make("Frame", {
	Name = "Monitor",
	Size = UDim2.fromScale(1, 1),
	BackgroundColor3 = WHITE,
})
rounded(card, 14)
make("UIGradient", { Rotation = 90, Color = ColorSequence.new(SKY, LILAC), Parent = card })
make("UIStroke", { Thickness = 1.5, Color = RIM, Transparency = 0.3, Parent = card })
inset(card, PAD, PAD)
list(card, Enum.FillDirection.Vertical, GAP)

local height = 2 * PAD - GAP
local function section(h: number)
	height += h + GAP
end

-- Header ---------------------------------------------------------------------

local HEADER = 34
local header = blank("Header", 1, UDim2.new(1, 0, 0, HEADER), card)
section(HEADER)
list(header, Enum.FillDirection.Horizontal, 10, { VerticalAlignment = Enum.VerticalAlignment.Center })

local beat = make("Frame", {
	Name = "Heartbeat",
	LayoutOrder = 1,
	Size = UDim2.fromOffset(10, 10),
	BackgroundColor3 = DOWN,
	Parent = header,
})
rounded(beat, 5)
make("UIStroke", { Thickness = 2, Color = WHITE, Transparency = 0.6, Parent = beat })

local titles = blank("Titles", 2, UDim2.new(0, 0, 1, 0), header)
fill(titles)
list(titles, Enum.FillDirection.Vertical, 0)
local title = label(titles, {
	Name = "Title",
	LayoutOrder = 1,
	Text = "System",
	FontFace = TITLE,
	TextSize = 17,
	Size = UDim2.new(1, 0, 0, 19),
})
edged(title)
local subtitle = label(titles, {
	Name = "Subtitle",
	LayoutOrder = 2,
	Text = "",
	TextSize = 12,
	TextTransparency = 0.15,
	TextTruncate = Enum.TextTruncate.AtEnd,
	Size = UDim2.new(1, 0, 0, 15),
})

--- A pill that is as wide as what it says.
local function pill(name: string, order: number): (Frame, TextLabel)
	local frame = make("Frame", {
		Name = name,
		LayoutOrder = order,
		AutomaticSize = Enum.AutomaticSize.X,
		Size = UDim2.new(0, 0, 0, 22),
		BackgroundColor3 = WHITE,
		BackgroundTransparency = 0.82,
		Parent = header,
	})
	rounded(frame, 11)
	inset(frame, 9, 0)
	list(frame, Enum.FillDirection.Horizontal, 6, { VerticalAlignment = Enum.VerticalAlignment.Center })
	local text = label(frame, {
		Name = "Text",
		LayoutOrder = 2,
		FontFace = MONO,
		TextSize = 11,
		AutomaticSize = Enum.AutomaticSize.X,
		Size = UDim2.new(0, 0, 1, 0),
	})
	return frame, text
end

-- A battery drawn from frames rather than a glyph, so it looks the same in
-- every face: a body with a fill whose width is the charge, and a nub.
local batteryPill, batteryText = pill("Battery", 3)
local cell = blank("Cell", 1, UDim2.fromOffset(20, 10), batteryPill)
local body = make("Frame", {
	Name = "Body",
	Size = UDim2.fromOffset(17, 10),
	BackgroundTransparency = 1,
	Parent = cell,
})
rounded(body, 2)
make("UIStroke", { Thickness = 1, Color = WHITE, Transparency = 0.1, Parent = body })
local charge = make("Frame", {
	Name = "Charge",
	Position = UDim2.fromOffset(2, 2),
	Size = UDim2.fromOffset(13, 6),
	BackgroundColor3 = WHITE,
	Parent = body,
})
rounded(charge, 1)
make("Frame", {
	Name = "Nub",
	Position = UDim2.fromOffset(18, 3),
	Size = UDim2.fromOffset(2, 4),
	BackgroundColor3 = WHITE,
	BackgroundTransparency = 0.1,
	Parent = cell,
})
batteryPill.Visible = false

local _, uptimeText = pill("Uptime", 4)

-- CPU ------------------------------------------------------------------------

local cpu = panel(card, "Cpu", 2)
local cores = #System.Cpu().cores

local top = cpu:row("Summary", 44)
list(top, Enum.FillDirection.Horizontal, 4, { VerticalAlignment = Enum.VerticalAlignment.Center })
local cpuValue = label(top, {
	Name = "Value",
	LayoutOrder = 1,
	FontFace = DISPLAY,
	TextSize = 40,
	AutomaticSize = Enum.AutomaticSize.X,
	Size = UDim2.new(0, 0, 1, 0),
})
edged(cpuValue)
label(top, {
	Name = "Percent",
	LayoutOrder = 2,
	Text = "%",
	FontFace = TITLE,
	TextSize = 18,
	TextTransparency = 0.15,
	TextYAlignment = Enum.TextYAlignment.Bottom,
	Size = UDim2.new(0, 18, 0, 36),
})
local about = blank("About", 3, UDim2.new(0, 0, 1, 0), top)
fill(about)
inset(about, 8, 0)
list(about, Enum.FillDirection.Vertical, 1, { VerticalAlignment = Enum.VerticalAlignment.Center })
caption(about, "PROCESSOR", 1).Size = UDim2.new(0, 0, 0, 12)
local brand = label(about, {
	Name = "Brand",
	LayoutOrder = 2,
	TextSize = 13,
	TextTruncate = Enum.TextTruncate.AtEnd,
	Size = UDim2.new(1, 0, 0, 15),
})
local clockText = label(about, {
	Name = "Detail",
	LayoutOrder = 3,
	FontFace = MONO,
	TextSize = 10,
	TextTransparency = 0.2,
	TextTruncate = Enum.TextTruncate.AtEnd,
	Size = UDim2.new(1, 0, 0, 13),
})

-- The graph sits over two guide lines. The lines are siblings of the bars'
-- frame rather than children, so the list lays out the bars and nothing else.
local graph = cpu:row("Graph", 52)
for i, at in { 0, 0.5 } do
	make("Frame", {
		Name = `Guide{i}`,
		Position = UDim2.fromScale(0, at),
		Size = UDim2.new(1, 0, 0, 1),
		BackgroundColor3 = WHITE,
		BackgroundTransparency = 0.82,
		Parent = graph,
	})
end
local cpuBars = bars(blank("Bars", 1, UDim2.fromScale(1, 1), graph), 60, Enum.VerticalAlignment.Bottom, COOL)

local coresHead = cpu:row("CoresHead", 12)
list(coresHead, Enum.FillDirection.Horizontal, 6)
local coresCaption = caption(coresHead, `CORES · {cores}`, 1)
local spacer = blank("Spacer", 2, UDim2.new(0, 0, 1, 0), coresHead)
fill(spacer)
local hottest = label(coresHead, {
	Name = "Hottest",
	LayoutOrder = 3,
	FontFace = MONO,
	TextSize = 10,
	TextTransparency = 0.2,
	AutomaticSize = Enum.AutomaticSize.X,
	Size = UDim2.new(0, 0, 1, 0),
})

-- SIXTEEN TILES, WHATEVER THE MACHINE. Up to sixteen threads each has its
-- own; past that each tile is the average of a run of neighbours, so a
-- 64-thread workstation is sixteen readable tiles rather than sixty-four
-- slivers. Unused tiles are built hidden and the grid skips them: the tree has
-- the same shape on every machine, and nothing is created after mount.
local TILES = 16
local group = math.ceil(cores / TILES)
local shownTiles = math.ceil(cores / group)
local columns = math.min(shownTiles, 8)
local TILE_GAP = 4
local tileWidth = math.floor((INNER - (columns - 1) * TILE_GAP) / columns)
local TILE_HEIGHT = 24
local rowsOfTiles = math.ceil(shownTiles / columns)
if group > 1 then
	coresCaption.Text = `CORES · {cores} · {group} PER TILE`
end
local grid = cpu:row("Cores", rowsOfTiles * TILE_HEIGHT + (rowsOfTiles - 1) * TILE_GAP)
make("UIGridLayout", {
	CellSize = UDim2.fromOffset(tileWidth, TILE_HEIGHT),
	CellPadding = UDim2.fromOffset(TILE_GAP, TILE_GAP),
	FillDirectionMaxCells = columns,
	SortOrder = Enum.SortOrder.LayoutOrder,
	HorizontalAlignment = Enum.HorizontalAlignment.Center,
	Parent = grid,
})
local tiles: { Frame } = {}
local tileText: { TextLabel } = {}
for i = 1, TILES do
	local tile = make("Frame", {
		Name = string.format("Tile%02d", i),
		LayoutOrder = i,
		Visible = i <= shownTiles,
		BackgroundColor3 = COOL,
		BackgroundTransparency = 0.8,
		Parent = grid,
	})
	rounded(tile, 5)
	tiles[i] = tile
	tileText[i] = label(tile, {
		Name = "Load",
		FontFace = MONO,
		TextSize = 10,
		TextXAlignment = Enum.TextXAlignment.Center,
		Size = UDim2.fromScale(1, 1),
	})
end
section(cpu.height)

-- Memory ---------------------------------------------------------------------

local memory = panel(card, "Memory", 3)

local memHead = memory:row("Head", 14)
list(memHead, Enum.FillDirection.Horizontal, 6, { VerticalAlignment = Enum.VerticalAlignment.Center })
caption(memHead, "MEMORY", 1)
fill(blank("Spacer", 2, UDim2.new(0, 0, 1, 0), memHead))
local memText = label(memHead, {
	Name = "Amount",
	LayoutOrder = 3,
	FontFace = MONO,
	TextSize = 11,
	AutomaticSize = Enum.AutomaticSize.X,
	Size = UDim2.new(0, 0, 1, 0),
})

-- TWO SEGMENTS, ONE RATIO EACH. Both ask for no width and grow by their
-- GrowRatio, so the layout splits the bar in proportion to the bytes. The
-- arithmetic is the flex layout's, not this file's.
local memBar = memory:row("Bar", 12)
list(memBar, Enum.FillDirection.Horizontal, 3)
local function segment(name: string, order: number, transparency: number): (Frame, UIFlexItem)
	local frame = make("Frame", {
		Name = name,
		LayoutOrder = order,
		Size = UDim2.new(0, 0, 1, 0),
		BackgroundColor3 = WHITE,
		BackgroundTransparency = transparency,
		Parent = memBar,
	})
	rounded(frame, 6)
	local flex = make("UIFlexItem", {
		FlexMode = Enum.UIFlexMode.Custom,
		GrowRatio = 1,
		ShrinkRatio = 0,
		Parent = frame,
	})
	return frame, flex
end
local usedSegment, usedFlex = segment("Used", 1, 0.05)
local _, freeFlex = segment("Free", 2, 0.78)

local memFoot = memory:row("Foot", 12)
list(memFoot, Enum.FillDirection.Horizontal, 6)
local memPercent = label(memFoot, {
	Name = "Percent",
	LayoutOrder = 1,
	FontFace = MONO,
	TextSize = 10,
	TextTransparency = 0.2,
	AutomaticSize = Enum.AutomaticSize.X,
	Size = UDim2.new(0, 0, 1, 0),
})
fill(blank("Spacer", 2, UDim2.new(0, 0, 1, 0), memFoot))
local swapText = label(memFoot, {
	Name = "Swap",
	LayoutOrder = 3,
	FontFace = MONO,
	TextSize = 10,
	TextTransparency = 0.2,
	AutomaticSize = Enum.AutomaticSize.X,
	Size = UDim2.new(0, 0, 1, 0),
})
section(memory.height)

-- Network --------------------------------------------------------------------

local network = panel(card, "Network", 4)

local netHead = network:row("Rates", 34)
list(netHead, Enum.FillDirection.Horizontal, 8, { HorizontalFlex = Enum.UIFlexAlignment.Fill })
local function rateColumn(name: string, order: number, colour: Color3, heading: string): TextLabel
	local column = blank(name, order, UDim2.new(0, 0, 1, 0), netHead)
	list(column, Enum.FillDirection.Vertical, 2)
	local head = blank("Head", 1, UDim2.new(1, 0, 0, 12), column)
	list(head, Enum.FillDirection.Horizontal, 5, { VerticalAlignment = Enum.VerticalAlignment.Center })
	local dot = make("Frame", {
		Name = "Swatch",
		LayoutOrder = 1,
		Size = UDim2.fromOffset(7, 7),
		BackgroundColor3 = colour,
		Parent = head,
	})
	rounded(dot, 4)
	caption(head, heading, 2)
	local value = label(column, {
		Name = "Rate",
		LayoutOrder = 2,
		FontFace = MONO,
		TextSize = 16,
		Size = UDim2.new(1, 0, 0, 20),
	})
	edged(value)
	return value
end
local downText = rateColumn("Down", 1, DOWN, "DOWNLOAD")
local upText = rateColumn("Up", 2, UP, "UPLOAD")

-- A WAVEFORM FROM TWO LISTS. The top half's bars stand on its bottom edge and
-- the lower half's hang from its top, so download and upload meet at one line.
local wave = network:row("Wave", 56)
local downHalf = blank("DownHalf", 1, UDim2.new(1, 0, 0.5, -1), wave)
local upHalf = blank("UpHalf", 2, UDim2.new(1, 0, 0.5, -1), wave)
upHalf.Position = UDim2.new(0, 0, 0.5, 1)
make("Frame", {
	Name = "Axis",
	Position = UDim2.new(0, 0, 0.5, -1),
	Size = UDim2.new(1, 0, 0, 1),
	BackgroundColor3 = WHITE,
	BackgroundTransparency = 0.6,
	Parent = wave,
})
local downBars = bars(downHalf, 60, Enum.VerticalAlignment.Bottom, DOWN)
local upBars = bars(upHalf, 60, Enum.VerticalAlignment.Top, UP)
local peakText = label(wave, {
	Name = "Peak",
	FontFace = MONO,
	TextSize = 9,
	TextTransparency = 0.25,
	TextXAlignment = Enum.TextXAlignment.Right,
	TextYAlignment = Enum.TextYAlignment.Top,
	Size = UDim2.new(1, 0, 0, 10),
	ZIndex = 2,
})
section(network.height)

-- Storage --------------------------------------------------------------------

local MAX_DISKS = 4
local function chosenDisks(): { any }
	local fixed, removable = {}, {}
	for _, disk in System.Disks() do
		table.insert(if disk.removable then removable else fixed, disk)
	end
	for _, disk in removable do
		table.insert(fixed, disk)
	end
	table.sort(fixed, function(a, b)
		return a.mount < b.mount
	end)
	local chosen = {}
	for i = 1, math.min(#fixed, MAX_DISKS) do
		chosen[i] = fixed[i]
	end
	return chosen
end

local disks = chosenDisks()
local storage = panel(card, "Storage", 5)
caption(storage:row("Head", 12), "STORAGE", 1)
type DiskRow = { fill: Frame, free: TextLabel, path: string }
local diskRows: { DiskRow } = {}
-- FOUR ROWS, SHOWN AS NEEDED, for the same reason as the tiles.
for i = 1, MAX_DISKS do
	local disk = disks[i]
	local row = storage:row(`Disk{i}`, 14, disk ~= nil)
	list(row, Enum.FillDirection.Horizontal, 8, { VerticalAlignment = Enum.VerticalAlignment.Center })
	label(row, {
		Name = "Mount",
		LayoutOrder = 1,
		Text = if disk then driveName(disk.mount) else "",
		FontFace = MONO,
		TextSize = 11,
		TextTruncate = Enum.TextTruncate.AtEnd,
		Size = UDim2.new(0, 64, 1, 0),
	})
	local track = make("Frame", {
		Name = "Track",
		LayoutOrder = 2,
		Size = UDim2.new(0, 0, 0, 6),
		BackgroundColor3 = WHITE,
		BackgroundTransparency = 0.8,
		Parent = row,
	})
	rounded(track, 3)
	fill(track)
	local used = make("Frame", {
		Name = "Used",
		Size = UDim2.fromScale(0, 1),
		BackgroundColor3 = COOL,
		Parent = track,
	})
	rounded(used, 3)
	local free = label(row, {
		Name = "Free",
		LayoutOrder = 3,
		FontFace = MONO,
		TextSize = 10,
		TextTransparency = 0.15,
		TextXAlignment = Enum.TextXAlignment.Right,
		Size = UDim2.new(0, 68, 1, 0),
	})
	diskRows[i] = { fill = used, free = free, path = if disk then disk.mount else "" }
end
label(storage:row("None", 14, #disks == 0), {
	Text = "No drives reported",
	TextTransparency = 0.3,
	Size = UDim2.fromScale(1, 1),
})
section(storage.height)

-- Processes ------------------------------------------------------------------

local SHOWN = 5
local sortBy = if desktop.Storage and desktop.Storage.get("monitor.processes") == "memory" then "memory" else "cpu"

local processes = panel(card, "Processes", 6)
local procHead = processes:row("Head", 18)
list(procHead, Enum.FillDirection.Horizontal, 6, { VerticalAlignment = Enum.VerticalAlignment.Center })
caption(procHead, "TOP PROCESSES", 1)
fill(blank("Spacer", 2, UDim2.new(0, 0, 1, 0), procHead))

-- A two-way switch: a list of two buttons inside one rounded track, the chosen
-- one filled white with dark type.
local switch = make("Frame", {
	Name = "Sort",
	LayoutOrder = 3,
	AutomaticSize = Enum.AutomaticSize.X,
	Size = UDim2.new(0, 0, 1, 0),
	BackgroundColor3 = WHITE,
	BackgroundTransparency = 0.82,
	Parent = procHead,
})
rounded(switch, 9)
inset(switch, 2, 2)
list(switch, Enum.FillDirection.Horizontal, 2)
local switchButtons: { [string]: TextButton } = {}

local procRows: { { name: TextLabel, count: TextLabel, value: TextLabel, meter: Frame } } = {}
for i = 1, SHOWN do
	local row = processes:row(`Process{i}`, 16)
	list(row, Enum.FillDirection.Horizontal, 8, { VerticalAlignment = Enum.VerticalAlignment.Center })
	local name = label(row, {
		Name = "Name",
		LayoutOrder = 1,
		TextSize = 13,
		TextTruncate = Enum.TextTruncate.AtEnd,
		Size = UDim2.new(0, 0, 1, 0),
	})
	fill(name)
	local count = label(row, {
		Name = "Count",
		LayoutOrder = 2,
		FontFace = MONO,
		TextSize = 9,
		TextTransparency = 0.35,
		AutomaticSize = Enum.AutomaticSize.X,
		Size = UDim2.new(0, 0, 1, 0),
	})
	local track = make("Frame", {
		Name = "Meter",
		LayoutOrder = 3,
		Size = UDim2.fromOffset(42, 4),
		BackgroundColor3 = WHITE,
		BackgroundTransparency = 0.8,
		Parent = row,
	})
	rounded(track, 2)
	local meter = make("Frame", {
		Name = "Fill",
		Size = UDim2.fromScale(0, 1),
		BackgroundColor3 = WHITE,
		Parent = track,
	})
	rounded(meter, 2)
	local value = label(row, {
		Name = "Value",
		LayoutOrder = 4,
		FontFace = MONO,
		TextSize = 11,
		TextXAlignment = Enum.TextXAlignment.Right,
		Size = UDim2.new(0, 58, 1, 0),
	})
	procRows[i] = { name = name, count = count, value = value, meter = meter }
end
section(processes.height)

-- Updating -------------------------------------------------------------------

local function processName(name: string): string
	return (string.gsub(name, "%.[eE][xX][eE]$", ""))
end

local function updateProcesses()
	local ranked = System.Processes(16)
	if sortBy == "memory" then
		table.sort(ranked, function(a, b)
			return a.memory > b.memory
		end)
	end
	local total = System.Memory().total
	for i, row in procRows do
		local p = ranked[i]
		if p then
			row.name.Text = processName(p.name)
			row.count.Text = if p.count > 1 then `×{p.count}` else ""
			if sortBy == "memory" then
				row.value.Text = bytes(p.memory)
				row.meter.Size = UDim2.fromScale(if total > 0 then p.memory / total else 0, 1)
				row.meter.BackgroundColor3 = WHITE
			else
				row.value.Text = string.format("%.1f%%", p.cpu)
				row.meter.Size = UDim2.fromScale(p.cpu / 100, 1)
				row.meter.BackgroundColor3 = heat(p.cpu)
			end
		else
			row.name.Text = ""
			row.count.Text = ""
			row.value.Text = ""
			row.meter.Size = UDim2.fromScale(0, 1)
		end
	end
	for key, button in switchButtons do
		local on = key == sortBy
		button.BackgroundTransparency = if on then 0 else 1
		button.TextColor3 = if on then INK else WHITE
	end
end

for i, option in { { key = "cpu", text = "CPU" }, { key = "memory", text = "MEM" } } do
	local button = make("TextButton", {
		Name = option.text,
		LayoutOrder = i,
		Text = option.text,
		FontFace = CAPS,
		TextSize = 9,
		AutoButtonColor = false,
		BackgroundColor3 = WHITE,
		Size = UDim2.new(0, 34, 1, 0),
		Parent = switch,
	})
	rounded(button, 7)
	switchButtons[option.key] = button
	button.MouseButton1Click:Connect(function()
		sortBy = option.key
		if desktop.Storage then
			desktop.Storage.set("monitor.processes", sortBy)
		end
		updateProcesses()
	end)
end

local beatOn = false
local function update()
	beatOn = not beatOn
	beat.BackgroundTransparency = if beatOn then 0 else 0.55

	local info = System.Info()
	subtitle.Text = if info.os ~= "" then info.os else "Unknown OS"
	uptimeText.Text = `UP {duration(info.uptime)}`

	local battery = System.Battery()
	batteryPill.Visible = battery ~= nil
	if battery then
		batteryText.Text = string.format("%d%%%s", math.floor(battery.percent + 0.5), if battery.charging then " +" else "")
		charge.Size = UDim2.fromOffset(math.max(1, math.floor(13 * battery.percent / 100 + 0.5)), 6)
		charge.BackgroundColor3 = if battery.percent <= 20 and not battery.plugged then HOT else WHITE
	end

	local c = System.Cpu()
	cpuValue.Text = string.format("%d", math.floor(c.usage + 0.5))
	brand.Text = if c.brand ~= "" then c.brand else "Processor"
	local threads = #c.cores
	local physical = c.physicalCores
	local detail = if physical and physical ~= threads then `{physical} cores · {threads} threads` else `{threads} cores`
	clockText.Text = if c.frequency > 0 then string.format("%.2f GHz · %s", c.frequency / 1000, detail) else detail
	plot(cpuBars, System.History("cpu"), 100, heat)

	local peak, peakAt = 0, 0
	for i, load in c.cores do
		if load > peak then
			peak, peakAt = load, i
		end
	end
	for i = 1, shownTiles do
		local sum, count = 0, 0
		for core = (i - 1) * group + 1, math.min(i * group, #c.cores) do
			sum += c.cores[core]
			count += 1
		end
		local load = if count > 0 then sum / count else 0
		tiles[i].BackgroundColor3 = heat(load)
		tiles[i].BackgroundTransparency = 0.78 - 0.73 * math.clamp(load / 100, 0, 1)
		tileText[i].Text = string.format("%d", math.floor(load + 0.5))
		tileText[i].TextColor3 = if load >= 40 then INK else WHITE
	end
	hottest.Text = if peakAt > 0 then string.format("hottest #%d · %d%%", peakAt, math.floor(peak + 0.5)) else ""

	local m = System.Memory()
	local usedRatio = if m.total > 0 then m.used / m.total else 0
	usedFlex.GrowRatio = math.max(m.used, 1)
	freeFlex.GrowRatio = math.max(m.total - m.used, 1)
	usedSegment.BackgroundColor3 = heat(usedRatio * 100)
	memText.Text = `{bytes(m.used)} / {bytes(m.total)}`
	memPercent.Text = string.format("%d%% used · %s free", math.floor(usedRatio * 100 + 0.5), bytes(m.available))
	swapText.Text = if m.swapTotal > 0 then `swap {bytes(m.swapUsed)} / {bytes(m.swapTotal)}` else ""

	local n = System.Network()
	downText.Text = rate(n.down)
	upText.Text = rate(n.up)
	local down, up = System.History("down"), System.History("up")
	-- ONE SCALE FOR BOTH HALVES, or a trickle of upload would look as tall as
	-- a download saturating the link. A floor keeps an idle line from turning
	-- background chatter into full-height bars.
	local scale = 16 * 1024
	for _, v in down do
		scale = math.max(scale, v)
	end
	for _, v in up do
		scale = math.max(scale, v)
	end
	plot(downBars, down, scale)
	plot(upBars, up, scale)
	peakText.Text = `peak {rate(scale)}`

	local byMount: { [string]: any } = {}
	for _, disk in System.Disks() do
		byMount[disk.mount] = disk
	end
	for _, row in diskRows do
		local disk = byMount[row.path]
		if disk and disk.total > 0 then
			local usedPart = 1 - disk.available / disk.total
			row.fill.Size = UDim2.fromScale(usedPart, 1)
			row.fill.BackgroundColor3 = heat(usedPart * 100)
			row.free.Text = `{bytes(disk.available)} free`
		end
	end

	updateProcesses()
end

update()
System.OnSample(update)

-- The window, sized to the panels above it.
local root = widget({
	width = WIDTH,
	height = height,
	anchor = "top-left",
	offset = { x = 24, y = 24 },
	draggable = true,
	savePosition = true,
	snapToEdges = true,
	keepOnScreen = true,
})
card.Parent = root

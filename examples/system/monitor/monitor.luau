--!strict
--[[
	System Monitor: what the machine is doing, in one dark panel.

	    cargo run -- examples/system/monitor
	    cargo run -- snapshot examples/system/monitor --after 6 -o monitor.png

	Laid out the way menu-bar monitors such as iStat Menus and Stats lay out
	their dropdowns: one solid surface, sections divided by hairlines, a grey
	label beside a white value, and colour only where there is data. The
	surface and type colours are the Dew dashboard's own tokens.

	THE DEW GRADIENT IS THE ACCENT AND NOTHING ELSE. Sky blue to periwinkle
	fills the CPU and core bars and the memory and drive meters, and its two
	ends are the two network series: download is the sky end, upload the
	periwinkle one. A value turns coral only past a threshold worth noticing.

	THE LAYOUT ENGINE DOES THE ARITHMETIC. Every row is a UIListLayout and its
	spare width goes to whichever child has a UIFlexItem. Graphs are lists with
	HorizontalFlex = Fill, so sixty bars share any width evenly. Memory is two
	segments whose GrowRatios are the bytes themselves.

	SAME TREE ON EVERY MACHINE. Sixteen core bars and four drive rows are built
	once; the unused ones are hidden and the lists skip them. Past sixteen
	threads each bar averages a run of neighbours.

	ONE REPAINT A SECOND. Everything updates in `System.OnSample`, so the panel
	does not show up in its own CPU graph.
]]

local widget = assert(desktop.Widget, "dew.toml must grant `widget`")
local System = assert(desktop.System, "dew.toml must grant `system`")

local WIDTH = 300
local PAD = 16

-- The dashboard's tokens (dashboard/dashboard.luau).
local SURFACE = Color3.fromRGB(18, 20, 26)
local TRACK = Color3.fromRGB(32, 36, 46)
local RULE = Color3.fromRGB(38, 42, 54)
local TEXT = Color3.fromRGB(233, 239, 249)
local MUTED = Color3.fromRGB(146, 162, 188)
local FAINT = Color3.fromRGB(96, 106, 124)

-- The accent: the clock's gradient, and a warning used sparingly.
local SKY = Color3.fromRGB(86, 164, 240)
local LILAC = Color3.fromRGB(150, 134, 236)
local CORAL = Color3.fromRGB(255, 112, 128)
local WHITE = Color3.new(1, 1, 1)

local function face(family: string, weight: Enum.FontWeight?): Font
	return Font.new(`rbxasset://fonts/families/{family}.json`, weight or Enum.FontWeight.Regular)
end
local LIGHT = face("SourceSansPro", Enum.FontWeight.Light)
local BODY = face("SourceSansPro")
local STRONG = face("SourceSansPro", Enum.FontWeight.SemiBold)
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

--- The drive's own name where it has one worth showing: `C:` rather than `C:\`.
local function driveName(path: string): string
	return string.match(path, "^(%a:)[\\/]?$") or path
end

local function processName(name: string): string
	return (string.gsub(name, "%.[eE][xX][eE]$", ""))
end

-- Building -------------------------------------------------------------------

--- An instance of `class` with `props`, parented last so it arrives whole.
local function make(class: string, props: { [string]: any }): any
	local instance = Instance.new(class)
	for key, value in props do
		if key ~= "Parent" then
			(instance :: any)[key] = value
		end
	end
	instance.Parent = props.Parent
	return instance
end

local function rounded(parent: Instance, radius: number)
	make("UICorner", { CornerRadius = UDim.new(0, radius), Parent = parent })
end

--- A UIListLayout in LayoutOrder. SortOrder defaults to Name on every layout,
--- which is rarely what the LayoutOrder beside it meant.
local function list(parent: Instance, direction: Enum.FillDirection, gap: number, props: { [string]: any }?)
	local layout = make("UIListLayout", {
		FillDirection = direction,
		SortOrder = Enum.SortOrder.LayoutOrder,
		Padding = UDim.new(0, gap),
		Parent = parent,
	})
	for key, value in props or {} do
		(layout :: any)[key] = value
	end
end

--- Take whatever width the row's other children leave.
local function grow(instance: Instance)
	make("UIFlexItem", { FlexMode = Enum.UIFlexMode.Fill, Parent = instance })
end

--- Paint `instance` with the accent: periwinkle at the top (or the left, at
--- rotation 0) to sky blue. Its own colour must be white for the gradient to
--- show true.
local function accent(instance: Instance, rotation: number?)
	make("UIGradient", {
		Rotation = rotation or 90,
		Color = ColorSequence.new(LILAC, SKY),
		Parent = instance,
	})
end

local function blank(parent: Instance, name: string, order: number, size: UDim2): Frame
	return make("Frame", {
		Name = name,
		LayoutOrder = order,
		Size = size,
		BackgroundTransparency = 1,
		Parent = parent,
	})
end

--- A label as wide as its text unless told otherwise.
local function text(parent: Instance, props: { [string]: any }): TextLabel
	local label = make("TextLabel", {
		Text = "",
		BackgroundTransparency = 1,
		TextColor3 = TEXT,
		FontFace = BODY,
		TextSize = 13,
		TextXAlignment = Enum.TextXAlignment.Left,
		TextYAlignment = Enum.TextYAlignment.Center,
		AutomaticSize = Enum.AutomaticSize.X,
		Size = UDim2.new(0, 0, 1, 0),
	})
	for key, value in props do
		(label :: any)[key] = value
	end
	label.Parent = parent
	return label
end

-- The panel ------------------------------------------------------------------

local card = make("Frame", {
	Name = "Monitor",
	Size = UDim2.fromScale(1, 1),
	BackgroundColor3 = SURFACE,
})
rounded(card, 12)
-- Inside the edge: the card fills the window, so a stroke outside it (the
-- engine's default) would be cut off along every straight side.
make("UIStroke", {
	Thickness = 1,
	Color = RULE,
	BorderStrokePosition = Enum.BorderStrokePosition.Inner,
	Parent = card,
})
make("UIPadding", {
	PaddingLeft = UDim.new(0, PAD),
	PaddingRight = UDim.new(0, PAD),
	PaddingTop = UDim.new(0, 14),
	PaddingBottom = UDim.new(0, 16),
	Parent = card,
})
list(card, Enum.FillDirection.Vertical, 0)

-- The window's height is the sum of the rows added below, so the window can
-- be sized before it exists.
local height = 14 + 16
local order = 0

--- A full-width row `size` pixels tall, next down the panel. A hidden row
--- takes no height, and the list skips it.
local function row(name: string, size: number, shown: boolean?): Frame
	order += 1
	local frame = blank(card, name, order, UDim2.new(1, 0, 0, size))
	if shown == false then
		frame.Visible = false
	else
		height += size
	end
	return frame
end

local function gap(size: number, shown: boolean?)
	row(`Gap{order + 1}`, size, shown)
end

--- A hairline between sections, with room either side.
local function rule()
	gap(12)
	local line = row(`Rule{order + 1}`, 1)
	line.BackgroundTransparency = 0
	line.BackgroundColor3 = RULE
	gap(12)
end

--- A spacer that takes a row's spare width.
local function spacer(parent: Instance, order: number)
	grow(blank(parent, "Spacer", order, UDim2.new(0, 0, 1, 0)))
end

--- A section's heading: its name on the left, a headline value on the right.
local function heading(name: string): TextLabel
	local frame = row(`{name}Heading`, 18)
	list(frame, Enum.FillDirection.Horizontal, 8, { VerticalAlignment = Enum.VerticalAlignment.Center })
	text(frame, { Name = "Title", LayoutOrder = 1, Text = name, FontFace = STRONG })
	spacer(frame, 2)
	return text(frame, { Name = "Value", LayoutOrder = 3, FontFace = MONO, TextSize = 12 })
end

--- A grey label and a white value, sharing the row with its siblings.
local function stat(parent: Instance, order: number, label: string): TextLabel
	local frame = blank(parent, label, order, UDim2.new(0, 0, 1, 0))
	grow(frame)
	list(frame, Enum.FillDirection.Horizontal, 6, { VerticalAlignment = Enum.VerticalAlignment.Center })
	text(frame, { Name = "Label", LayoutOrder = 1, Text = label, TextColor3 = MUTED, TextSize = 12 })
	return text(frame, { Name = "Value", LayoutOrder = 2, FontFace = MONO, TextSize = 11 })
end

--- `count` bars sharing `parent`'s width, standing on `align`'s edge.
--- HorizontalFlex = Fill does the sharing: each bar asks for 1px and the list
--- hands each the same part of what is left. With no `colour`, the accent.
local function bars(parent: Instance, count: number, align: Enum.VerticalAlignment, colour: Color3?): { Frame }
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
			BackgroundColor3 = colour or WHITE,
			BackgroundTransparency = 1,
			Parent = parent,
		})
		rounded(bar, 1)
		if not colour then
			accent(bar)
		end
		made[i] = bar
	end
	return made
end

--- Set `row` from `series`, newest on the right. A sample not taken yet is
--- hidden rather than drawn as zero: a minute that has not happened is not idle.
local function plot(row: { Frame }, series: { number }, scale: number)
	local offset = #row - #series
	for i, bar in row do
		local value = series[i - offset]
		if value == nil then
			bar.BackgroundTransparency = 1
		else
			bar.Size = UDim2.new(0, 1, math.clamp(value / scale, 0, 1), 1)
			bar.BackgroundTransparency = 0
		end
	end
end

-- Header ---------------------------------------------------------------------

local header = row("Header", 18)
list(header, Enum.FillDirection.Horizontal, 8, { VerticalAlignment = Enum.VerticalAlignment.Center })
local beat = make("Frame", {
	Name = "Heartbeat",
	LayoutOrder = 1,
	Size = UDim2.fromOffset(6, 6),
	BackgroundColor3 = SKY,
	Parent = header,
})
rounded(beat, 3)
local osName = text(header, {
	Name = "Os",
	LayoutOrder = 2,
	TextColor3 = MUTED,
	TextSize = 12,
	TextTruncate = Enum.TextTruncate.AtEnd,
	AutomaticSize = Enum.AutomaticSize.None,
})
grow(osName)

-- A battery drawn from frames rather than a glyph, so it is the same in every
-- face: an outline, a fill whose width is the charge, and a nub.
local battery = blank(header, "Battery", 3, UDim2.new(0, 0, 1, 0))
battery.AutomaticSize = Enum.AutomaticSize.X
list(battery, Enum.FillDirection.Horizontal, 5, { VerticalAlignment = Enum.VerticalAlignment.Center })
local cell = blank(battery, "Cell", 1, UDim2.fromOffset(19, 9))
local shell = make("Frame", { Name = "Shell", Size = UDim2.fromOffset(16, 9), BackgroundTransparency = 1, Parent = cell })
rounded(shell, 2)
make("UIStroke", {
	Thickness = 1,
	Color = MUTED,
	BorderStrokePosition = Enum.BorderStrokePosition.Inner,
	Parent = shell,
})
local charge = make("Frame", {
	Name = "Charge",
	Position = UDim2.fromOffset(2, 2),
	Size = UDim2.fromOffset(12, 5),
	BackgroundColor3 = TEXT,
	Parent = shell,
})
rounded(charge, 1)
make("Frame", {
	Name = "Nub",
	Position = UDim2.fromOffset(17, 3),
	Size = UDim2.fromOffset(2, 3),
	BackgroundColor3 = MUTED,
	Parent = cell,
})
local batteryText = text(battery, { Name = "Percent", LayoutOrder = 2, FontFace = MONO, TextSize = 11, TextColor3 = MUTED })
battery.Visible = false

local uptime = text(header, { Name = "Uptime", LayoutOrder = 4, FontFace = MONO, TextSize = 11, TextColor3 = MUTED })

rule()

-- CPU ------------------------------------------------------------------------

-- The headline: a light, large number, readable from across the room, with
-- the processor's name and shape beside it, smaller.
local summary = row("Cpu", 40)
list(summary, Enum.FillDirection.Horizontal, 12, { VerticalAlignment = Enum.VerticalAlignment.Center })
local cpuValue = text(summary, { Name = "Value", LayoutOrder = 1, FontFace = LIGHT, TextSize = 38 })
local about = blank(summary, "About", 2, UDim2.new(0, 0, 0, 34))
grow(about)
list(about, Enum.FillDirection.Vertical, 2)
local brand = text(about, {
	Name = "Brand",
	LayoutOrder = 1,
	FontFace = STRONG,
	TextTruncate = Enum.TextTruncate.AtEnd,
	AutomaticSize = Enum.AutomaticSize.None,
	Size = UDim2.new(1, 0, 0, 16),
})
local detail = text(about, {
	Name = "Detail",
	LayoutOrder = 2,
	TextColor3 = MUTED,
	TextSize = 12,
	TextTruncate = Enum.TextTruncate.AtEnd,
	AutomaticSize = Enum.AutomaticSize.None,
	Size = UDim2.new(1, 0, 0, 15),
})

gap(10)

-- A minute of load over a half-way guide and a baseline. The guides are
-- siblings of the bars' frame, so the list lays out bars and nothing else.
local graph = row("Graph", 48)
for i, at in { 0.5, 1 } do
	make("Frame", {
		Name = `Guide{i}`,
		Position = UDim2.new(0, 0, at, -1),
		Size = UDim2.new(1, 0, 0, 1),
		BackgroundColor3 = RULE,
		Parent = graph,
	})
end
local cpuBars = bars(blank(graph, "Bars", 1, UDim2.fromScale(1, 1)), 60, Enum.VerticalAlignment.Bottom)

gap(12)

local coresHeading = row("CoresHeading", 14)
list(coresHeading, Enum.FillDirection.Horizontal, 6, { VerticalAlignment = Enum.VerticalAlignment.Center })
local coresLabel = text(coresHeading, { Name = "Label", LayoutOrder = 1, TextColor3 = MUTED, TextSize = 12 })
spacer(coresHeading, 2)
local busiest = text(coresHeading, { Name = "Busiest", LayoutOrder = 3, FontFace = MONO, TextSize = 11, TextColor3 = MUTED })

gap(6)

-- One bar per core, like an equaliser: a track filled up from its foot.
-- Sixteen always exist; see the header comment.
local cores = #System.Cpu().cores
local CELLS = 16
local group = math.ceil(cores / CELLS)
local shownCells = math.ceil(cores / group)
coresLabel.Text = if group > 1 then `{cores} threads, {group} per bar` else `{cores} threads`

local coreRow = row("Cores", 26)
list(coreRow, Enum.FillDirection.Horizontal, 3, { HorizontalFlex = Enum.UIFlexAlignment.Fill })
local coreFills: { Frame } = {}
for i = 1, CELLS do
	local track = make("Frame", {
		Name = string.format("Tile%02d", i),
		LayoutOrder = i,
		Visible = i <= shownCells,
		Size = UDim2.new(0, 1, 1, 0),
		BackgroundColor3 = TRACK,
		Parent = coreRow,
	})
	rounded(track, 2)
	local fill = make("Frame", {
		Name = "Load",
		AnchorPoint = Vector2.new(0, 1),
		Position = UDim2.fromScale(0, 1),
		Size = UDim2.fromScale(1, 0),
		BackgroundColor3 = WHITE,
		Parent = track,
	})
	rounded(fill, 2)
	accent(fill)
	coreFills[i] = fill
end

rule()

-- Memory ---------------------------------------------------------------------

local memValue = heading("Memory")
gap(10)

-- TWO SEGMENTS, ONE RATIO EACH. Both ask for no width and grow by their
-- GrowRatio, so the bar is split in proportion to the bytes without a line of
-- arithmetic here.
local memBar = row("MemoryBar", 6)
list(memBar, Enum.FillDirection.Horizontal, 2)
local function segment(name: string, order: number, colour: Color3): (Frame, UIFlexItem)
	local frame = make("Frame", {
		Name = name,
		LayoutOrder = order,
		Size = UDim2.new(0, 0, 1, 0),
		BackgroundColor3 = colour,
		Parent = memBar,
	})
	rounded(frame, 3)
	local flex = make("UIFlexItem", { FlexMode = Enum.UIFlexMode.Custom, GrowRatio = 1, ShrinkRatio = 0, Parent = frame })
	return frame, flex
end
local usedSegment, usedFlex = segment("Used", 1, WHITE)
accent(usedSegment, 0)
local _, freeFlex = segment("Free", 2, TRACK)

gap(10)
local memStats = row("MemoryStats", 16)
list(memStats, Enum.FillDirection.Horizontal, 16)
local memUsed = stat(memStats, 1, "Used")
local memFree = stat(memStats, 2, "Free")
local memSwap = stat(memStats, 3, "Swap")

rule()

-- Network --------------------------------------------------------------------

heading("Network")
gap(10)

local rates = row("Rates", 16)
list(rates, Enum.FillDirection.Horizontal, 16)
local function legend(order: number, name: string, colour: Color3): TextLabel
	local frame = blank(rates, name, order, UDim2.new(0, 0, 1, 0))
	grow(frame)
	list(frame, Enum.FillDirection.Horizontal, 6, { VerticalAlignment = Enum.VerticalAlignment.Center })
	local swatch = make("Frame", {
		Name = "Swatch",
		LayoutOrder = 1,
		Size = UDim2.fromOffset(6, 6),
		BackgroundColor3 = colour,
		Parent = frame,
	})
	rounded(swatch, 3)
	text(frame, { Name = "Label", LayoutOrder = 2, Text = name, TextColor3 = MUTED, TextSize = 12 })
	spacer(frame, 3)
	return text(frame, { Name = "Rate", LayoutOrder = 4, FontFace = MONO, TextSize = 11 })
end
local downText = legend(1, "Down", SKY)
local upText = legend(2, "Up", LILAC)

gap(10)

-- A WAVEFORM FROM TWO LISTS: download stands on the centre line and upload
-- hangs from it, both against one scale, in the gradient's two colours.
local wave = row("Wave", 44)
local downHalf = blank(wave, "DownHalf", 1, UDim2.new(1, 0, 0.5, 0))
local upHalf = blank(wave, "UpHalf", 2, UDim2.new(1, 0, 0.5, 0))
upHalf.Position = UDim2.fromScale(0, 0.5)
make("Frame", {
	Name = "Axis",
	Position = UDim2.fromScale(0, 0.5),
	Size = UDim2.new(1, 0, 0, 1),
	BackgroundColor3 = RULE,
	ZIndex = 2,
	Parent = wave,
})
local downBars = bars(downHalf, 60, Enum.VerticalAlignment.Bottom, SKY)
local upBars = bars(upHalf, 60, Enum.VerticalAlignment.Top, LILAC)

gap(6)
local scaleRow = row("Scale", 14)
list(scaleRow, Enum.FillDirection.Horizontal, 6)
text(scaleRow, { Name = "Window", LayoutOrder = 1, Text = "last 60 s", FontFace = MONO, TextSize = 10, TextColor3 = FAINT })
spacer(scaleRow, 2)
local peakText = text(scaleRow, { Name = "Peak", LayoutOrder = 3, FontFace = MONO, TextSize = 10, TextColor3 = FAINT })

rule()

-- Storage --------------------------------------------------------------------

local MAX_DISKS = 4
local disks = System.Disks()
table.sort(disks, function(a, b)
	if a.removable ~= b.removable then
		return not a.removable
	end
	return a.mount < b.mount
end)

heading("Storage")
type DiskRow = { fill: Frame, free: TextLabel, path: string }
local diskRows: { DiskRow } = {}
-- FOUR ROWS, SHOWN AS NEEDED, for the same reason as the core bars.
for i = 1, MAX_DISKS do
	local disk = disks[i]
	gap(8, disk ~= nil)
	local frame = row(`Disk{i}`, 16, disk ~= nil)
	list(frame, Enum.FillDirection.Horizontal, 10, { VerticalAlignment = Enum.VerticalAlignment.Center })
	text(frame, {
		Name = "Mount",
		LayoutOrder = 1,
		Text = if disk then driveName(disk.mount) else "",
		TextColor3 = MUTED,
		TextSize = 12,
		TextTruncate = Enum.TextTruncate.AtEnd,
		AutomaticSize = Enum.AutomaticSize.None,
		Size = UDim2.new(0, 56, 1, 0),
	})
	local track = make("Frame", {
		Name = "Track",
		LayoutOrder = 2,
		Size = UDim2.new(0, 0, 0, 4),
		BackgroundColor3 = TRACK,
		Parent = frame,
	})
	rounded(track, 2)
	grow(track)
	local used = make("Frame", { Name = "Used", Size = UDim2.fromScale(0, 1), BackgroundColor3 = WHITE, Parent = track })
	rounded(used, 2)
	accent(used, 0)
	local free = text(frame, {
		Name = "Free",
		LayoutOrder = 3,
		FontFace = MONO,
		TextSize = 11,
		TextXAlignment = Enum.TextXAlignment.Right,
		AutomaticSize = Enum.AutomaticSize.None,
		Size = UDim2.new(0, 64, 1, 0),
	})
	diskRows[i] = { fill = used, free = free, path = if disk then disk.mount else "" }
end
gap(8, #disks == 0)
local noDisks = row("NoDisks", 16, #disks == 0)
text(noDisks, { Name = "Text", Text = "No drives reported", TextColor3 = MUTED, TextSize = 12 })

rule()

-- Processes ------------------------------------------------------------------

local SHOWN = 5
local sortBy = if desktop.Storage and desktop.Storage.get("monitor.processes") == "memory" then "memory" else "cpu"

local procHeading = row("ProcessesHeading", 18)
list(procHeading, Enum.FillDirection.Horizontal, 12, { VerticalAlignment = Enum.VerticalAlignment.Center })
text(procHeading, { Name = "Title", LayoutOrder = 1, Text = "Processes", FontFace = STRONG })
spacer(procHeading, 2)

-- Two words, not a pill: the chosen one white with the accent under it, the
-- other faint. Each is a button as wide as its word.
type Choice = { button: TextButton, mark: Frame }
local choices: { [string]: Choice } = {}

type ProcessRow = { name: TextLabel, count: TextLabel, value: TextLabel }
local procRows: { ProcessRow } = {}
for i = 1, SHOWN do
	gap(if i == 1 then 8 else 4)
	local frame = row(`Process{i}`, 16)
	list(frame, Enum.FillDirection.Horizontal, 6, { VerticalAlignment = Enum.VerticalAlignment.Center })
	local name = text(frame, {
		Name = "Name",
		LayoutOrder = 1,
		TextTruncate = Enum.TextTruncate.AtEnd,
		AutomaticSize = Enum.AutomaticSize.None,
	})
	grow(name)
	local count = text(frame, { Name = "Count", LayoutOrder = 2, FontFace = MONO, TextSize = 10, TextColor3 = FAINT })
	local value = text(frame, {
		Name = "Value",
		LayoutOrder = 3,
		FontFace = MONO,
		TextSize = 11,
		TextXAlignment = Enum.TextXAlignment.Right,
		AutomaticSize = Enum.AutomaticSize.None,
		Size = UDim2.new(0, 60, 1, 0),
	})
	procRows[i] = { name = name, count = count, value = value }
end

-- Updating -------------------------------------------------------------------

local function updateProcesses()
	local ranked = System.Processes(16)
	if sortBy == "memory" then
		table.sort(ranked, function(a, b)
			return a.memory > b.memory
		end)
	end
	for i, r in procRows do
		local p = ranked[i]
		r.name.Text = if p then processName(p.name) else ""
		r.count.Text = if p and p.count > 1 then `×{p.count}` else ""
		if not p then
			r.value.Text = ""
		elseif sortBy == "memory" then
			r.value.Text = bytes(p.memory)
			r.value.TextColor3 = TEXT
		else
			r.value.Text = string.format("%.1f%%", p.cpu)
			r.value.TextColor3 = if p.cpu >= 50 then CORAL else TEXT
		end
	end
	for key, choice in choices do
		local on = key == sortBy
		choice.button.TextColor3 = if on then TEXT else FAINT
		choice.mark.Visible = on
	end
end

for i, option in { { key = "cpu", text = "CPU" }, { key = "memory", text = "MEM" } } do
	local button = make("TextButton", {
		Name = option.text,
		LayoutOrder = 2 + i,
		Text = option.text,
		FontFace = STRONG,
		TextSize = 11,
		AutoButtonColor = false,
		BackgroundTransparency = 1,
		AutomaticSize = Enum.AutomaticSize.X,
		Size = UDim2.new(0, 0, 1, 0),
		Parent = procHeading,
	})
	local mark = make("Frame", {
		Name = "Mark",
		AnchorPoint = Vector2.new(0, 1),
		Position = UDim2.fromScale(0, 1),
		Size = UDim2.new(1, 0, 0, 2),
		BackgroundColor3 = WHITE,
		Parent = button,
	})
	rounded(mark, 1)
	accent(mark, 0)
	choices[option.key] = { button = button, mark = mark }
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
	beat.BackgroundColor3 = if beatOn then SKY else LILAC

	local info = System.Info()
	osName.Text = if info.os ~= "" then info.os else "System"
	uptime.Text = `up {duration(info.uptime)}`

	local power = System.Battery()
	battery.Visible = power ~= nil
	if power then
		batteryText.Text = string.format("%d%%", math.floor(power.percent + 0.5))
		local low = power.percent <= 20 and not power.plugged
		charge.Size = UDim2.fromOffset(math.max(1, math.floor(12 * power.percent / 100 + 0.5)), 5)
		charge.BackgroundColor3 = if low then CORAL elseif power.charging then SKY else TEXT
	end

	local c = System.Cpu()
	cpuValue.Text = string.format("%d%%", math.floor(c.usage + 0.5))
	cpuValue.TextColor3 = if c.usage >= 85 then CORAL else TEXT
	brand.Text = if c.brand ~= "" then c.brand else "Processor"
	local threads = #c.cores
	local physical = c.physicalCores
	local shape = if physical and physical ~= threads then `{physical} cores, {threads} threads` else `{threads} cores`
	detail.Text = if c.frequency > 0 then string.format("%.2f GHz · %s", c.frequency / 1000, shape) else shape
	plot(cpuBars, System.History("cpu"), 100)

	local peak, peakAt = 0, 0
	for i, load in c.cores do
		if load > peak then
			peak, peakAt = load, i
		end
	end
	busiest.Text = if peakAt > 0 then string.format("busiest #%d  %d%%", peakAt, math.floor(peak + 0.5)) else ""
	for i = 1, shownCells do
		local sum, count = 0, 0
		for core = (i - 1) * group + 1, math.min(i * group, #c.cores) do
			sum += c.cores[core]
			count += 1
		end
		local load = if count > 0 then sum / count else 0
		coreFills[i].Size = UDim2.new(1, 0, math.clamp(load / 100, 0, 1), 0)
	end

	local m = System.Memory()
	local usedRatio = if m.total > 0 then m.used / m.total else 0
	usedFlex.GrowRatio = math.max(m.used, 1)
	freeFlex.GrowRatio = math.max(m.total - m.used, 1)
	memValue.Text = `{bytes(m.used)} / {bytes(m.total)}`
	memUsed.Text = string.format("%d%%", math.floor(usedRatio * 100 + 0.5))
	memUsed.TextColor3 = if usedRatio >= 0.9 then CORAL else TEXT
	memFree.Text = bytes(m.available)
	memSwap.Text = if m.swapTotal > 0 then bytes(m.swapUsed) else "none"

	local n = System.Network()
	downText.Text = rate(n.down)
	upText.Text = rate(n.up)
	local down, up = System.History("down"), System.History("up")
	-- ONE SCALE FOR BOTH HALVES, or a trickle of upload would stand as tall as
	-- a saturated download. A floor keeps background chatter from filling it.
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

	local byPath: { [string]: any } = {}
	for _, disk in System.Disks() do
		byPath[disk.mount] = disk
	end
	for _, r in diskRows do
		local disk = byPath[r.path]
		if disk and disk.total > 0 then
			local usedPart = 1 - disk.available / disk.total
			r.fill.Size = UDim2.fromScale(usedPart, 1)
			r.free.Text = bytes(disk.available)
			r.free.TextColor3 = if usedPart >= 0.9 then CORAL else TEXT
		end
	end

	updateProcesses()
end

update()
System.OnSample(update)

-- The window, sized to the rows above it.
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

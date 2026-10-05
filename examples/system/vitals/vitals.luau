--!strict
--[[
	Vitals: the desktop clock's companion.

	    cargo run -- examples/system/vitals
	    cargo run -- snapshot examples/system/vitals --after 6 -o vitals.png

	The clock's size, docked 8px under it by default, on the same dark surface
	as the system monitor. Three vitals share the width: CPU, memory, and
	network download. The Dew gradient is only the accent on the data.

	Click anywhere to switch between METERS (a bar each) and GRAPHS (the last
	twenty samples each). The choice is kept in `desktop.Storage`, the way the
	clock keeps 12h/24h.

	THREE COLUMNS, NO ARITHMETIC. The row is a UIListLayout whose columns each
	have a UIFlexItem, so they split whatever is left after the two dividers.
	Each graph is a list with HorizontalFlex = Fill, so twenty bars share a
	column of any width.
]]

local widget = assert(desktop.Widget, "dew.toml must grant `widget`")
local System = assert(desktop.System, "dew.toml must grant `system`")

local root = widget({
	width = 200,
	height = 44,
	anchor = "top-right",
	-- The clock sits at 24px and is 44px tall; this is 8px under it.
	offset = { x = 24, y = 76 },
	draggable = true,
	savePosition = true,
	snapToEdges = true,
	keepOnScreen = true,
})

-- The Dew dashboard's tokens, as the system monitor uses them.
local SURFACE = Color3.fromRGB(18, 20, 26)
local TRACK = Color3.fromRGB(32, 36, 46)
local RULE = Color3.fromRGB(38, 42, 54)
local TEXT = Color3.fromRGB(233, 239, 249)
local MUTED = Color3.fromRGB(146, 162, 188)
local SKY = Color3.fromRGB(86, 164, 240)
local LILAC = Color3.fromRGB(150, 134, 236)
local CORAL = Color3.fromRGB(255, 112, 128)
local WHITE = Color3.new(1, 1, 1)
local SAMPLES = 20

local LABEL = Font.new("rbxasset://fonts/families/SourceSansPro.json", Enum.FontWeight.SemiBold)
local MONO = Font.new("rbxasset://fonts/families/RobotoMono.json", Enum.FontWeight.Regular)

local graphs = desktop.Storage ~= nil and desktop.Storage.get("vitals.mode") == "graphs"

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

--- The accent, periwinkle to sky. `instance` must be white to show it true.
local function accent(instance: Instance, rotation: number)
	make("UIGradient", { Rotation = rotation, Color = ColorSequence.new(LILAC, SKY), Parent = instance })
end

--- A rate in at most four characters, for a column 50px wide.
local function compact(n: number): string
	local units = { "B", "K", "M", "G" }
	local i = 1
	while n >= 1000 and i < #units do
		n /= 1024
		i += 1
	end
	if i > 1 and n < 10 then
		return string.format("%.1f%s", n, units[i])
	end
	return string.format("%d%s", math.floor(n + 0.5), units[i])
end

-- A full-size button is the surface, as on the clock, so a click anywhere
-- reaches it.
local surface = make("TextButton", {
	Name = "VitalsSurface",
	Size = UDim2.fromScale(1, 1),
	BackgroundColor3 = SURFACE,
	Text = "",
	AutoButtonColor = false,
	Parent = root,
})
make("UICorner", { CornerRadius = UDim.new(0, 10), Parent = surface })
-- Around the box, not the glyphs: on a TextButton the default, Contextual,
-- outlines the text, and this one has none. Inside the edge, because the
-- surface fills the window and a stroke outside it (the engine's default)
-- would be cut off along every straight side.
make("UIStroke", {
	Thickness = 1,
	Color = RULE,
	ApplyStrokeMode = Enum.ApplyStrokeMode.Border,
	BorderStrokePosition = Enum.BorderStrokePosition.Inner,
	Parent = surface,
})
make("UIPadding", {
	PaddingLeft = UDim.new(0, 12),
	PaddingRight = UDim.new(0, 12),
	PaddingTop = UDim.new(0, 7),
	PaddingBottom = UDim.new(0, 7),
	Parent = surface,
})
list(surface, Enum.FillDirection.Horizontal, 9, { VerticalAlignment = Enum.VerticalAlignment.Center })

type Vital = { value: TextLabel, meter: Frame, fill: Frame, spark: Frame, bars: { Frame } }

local function divider(order: number)
	make("Frame", {
		Name = `Divider{order}`,
		LayoutOrder = order,
		Size = UDim2.new(0, 1, 0, 24),
		BackgroundColor3 = RULE,
		Parent = surface,
	})
end

local function vital(name: string, order: number, caption: string, colour: Color3?): Vital
	local column = make("Frame", {
		Name = name,
		LayoutOrder = order,
		Size = UDim2.new(0, 0, 1, 0),
		BackgroundTransparency = 1,
		Parent = surface,
	})
	make("UIFlexItem", { FlexMode = Enum.UIFlexMode.Fill, Parent = column })
	list(column, Enum.FillDirection.Vertical, 4)

	local head = make("Frame", {
		Name = "Head",
		LayoutOrder = 1,
		Size = UDim2.new(1, 0, 0, 14),
		BackgroundTransparency = 1,
		Parent = column,
	})
	make("TextLabel", {
		Name = "Caption",
		Text = caption,
		FontFace = LABEL,
		TextSize = 11,
		TextColor3 = MUTED,
		TextXAlignment = Enum.TextXAlignment.Left,
		BackgroundTransparency = 1,
		Size = UDim2.fromScale(1, 1),
		Parent = head,
	})
	local value = make("TextLabel", {
		Name = "Value",
		Text = "",
		FontFace = MONO,
		TextSize = 11,
		TextColor3 = TEXT,
		TextXAlignment = Enum.TextXAlignment.Right,
		BackgroundTransparency = 1,
		Size = UDim2.fromScale(1, 1),
		Parent = head,
	})

	-- METERS AND GRAPHS ARE BOTH BUILT, and a click only swaps which is
	-- Visible. Nothing is created or destroyed after mount.
	local body = make("Frame", {
		Name = "Body",
		LayoutOrder = 2,
		Size = UDim2.new(1, 0, 0, 12),
		BackgroundTransparency = 1,
		Parent = column,
	})
	local meter = make("Frame", {
		Name = "Meter",
		AnchorPoint = Vector2.new(0, 1),
		Position = UDim2.fromScale(0, 1),
		Size = UDim2.new(1, 0, 0, 4),
		BackgroundColor3 = TRACK,
		Parent = body,
	})
	make("UICorner", { CornerRadius = UDim.new(0, 2), Parent = meter })
	local fill = make("Frame", {
		Name = "Fill",
		Size = UDim2.fromScale(0, 1),
		BackgroundColor3 = colour or WHITE,
		Parent = meter,
	})
	make("UICorner", { CornerRadius = UDim.new(0, 2), Parent = fill })
	if not colour then
		accent(fill, 0)
	end

	local spark = make("Frame", {
		Name = "Spark",
		Size = UDim2.fromScale(1, 1),
		BackgroundTransparency = 1,
		Parent = body,
	})
	list(spark, Enum.FillDirection.Horizontal, 1, {
		HorizontalFlex = Enum.UIFlexAlignment.Fill,
		VerticalAlignment = Enum.VerticalAlignment.Bottom,
	})
	local bars = {}
	for i = 1, SAMPLES do
		local bar = make("Frame", {
			Name = string.format("Bar%02d", i),
			LayoutOrder = i,
			Size = UDim2.new(0, 1, 0, 1),
			BackgroundColor3 = colour or WHITE,
			BackgroundTransparency = 1,
			Parent = spark,
		})
		if not colour then
			accent(bar, 90)
		end
		bars[i] = bar
	end
	return { value = value, meter = meter, fill = fill, spark = spark, bars = bars }
end

local cpu = vital("Cpu", 1, "CPU")
divider(2)
local ram = vital("Memory", 3, "RAM")
divider(4)
-- Download, in the colour the monitor gives it.
local net = vital("Network", 5, "NET", SKY)
local vitals = { cpu, ram, net }

local function show()
	for _, v in vitals do
		v.meter.Visible = not graphs
		v.spark.Visible = graphs
	end
end

--- The last `SAMPLES` of `series` against `scale`, newest on the right. A
--- sample not taken yet is hidden rather than drawn as zero.
local function spark(v: Vital, series: { number }, scale: number)
	local skip = #series - SAMPLES
	for i, bar in v.bars do
		local value = series[i + skip]
		if value == nil then
			bar.BackgroundTransparency = 1
		else
			bar.Size = UDim2.new(0, 1, math.clamp(value / scale, 0, 1), 1)
			bar.BackgroundTransparency = 0
		end
	end
end

local function update()
	local c = System.Cpu()
	cpu.value.Text = string.format("%d%%", math.floor(c.usage + 0.5))
	cpu.value.TextColor3 = if c.usage >= 85 then CORAL else TEXT
	cpu.fill.Size = UDim2.fromScale(c.usage / 100, 1)
	spark(cpu, System.History("cpu"), 100)

	local m = System.Memory()
	local used = if m.total > 0 then m.used / m.total * 100 else 0
	ram.value.Text = string.format("%d%%", math.floor(used + 0.5))
	ram.value.TextColor3 = if used >= 90 then CORAL else TEXT
	ram.fill.Size = UDim2.fromScale(used / 100, 1)
	spark(ram, System.History("memory"), 100)

	local down = System.History("down")
	local peak = 16 * 1024
	for _, v in down do
		peak = math.max(peak, v)
	end
	local n = System.Network()
	net.value.Text = compact(n.down)
	net.fill.Size = UDim2.fromScale(math.clamp(n.down / peak, 0, 1), 1)
	spark(net, down, peak)
end

surface.MouseButton1Click:Connect(function()
	graphs = not graphs
	if desktop.Storage then
		desktop.Storage.set("vitals.mode", if graphs then "graphs" else "meters")
	end
	show()
end)

show()
update()
System.OnSample(update)

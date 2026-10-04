--!strict
--[[
	Vitals: the desktop clock's companion.

	    cargo run -- examples/system/vitals
	    cargo run -- snapshot examples/system/vitals --after 6 -o vitals.png

	The clock's size, colours and border, docked 8px under it by default, so the
	two read as one stack in the corner. Three vitals share the width: CPU,
	memory, and network download.

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

local WHITE = Color3.fromRGB(255, 255, 255)
local INK = Color3.fromRGB(30, 48, 80)
local COOL = Color3.fromRGB(196, 250, 236)
local WARM = Color3.fromRGB(255, 212, 120)
local HOT = Color3.fromRGB(255, 118, 140)
local SAMPLES = 20

local CAPS = Font.new("rbxasset://fonts/families/Montserrat.json", Enum.FontWeight.Bold)
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

-- The clock's surface, exactly: a full-size button under a vertical gradient.
local surface = make("TextButton", {
	Name = "VitalsSurface",
	Size = UDim2.fromScale(1, 1),
	BackgroundColor3 = WHITE,
	Text = "",
	AutoButtonColor = false,
	Parent = root,
})
make("UICorner", { CornerRadius = UDim.new(0, 10), Parent = surface })
make("UIGradient", {
	Rotation = 90,
	Color = ColorSequence.new(Color3.fromRGB(72, 148, 226), Color3.fromRGB(132, 120, 218)),
	Parent = surface,
})
make("UIStroke", { Thickness = 1.5, Color = Color3.fromRGB(210, 232, 255), Transparency = 0.3, Parent = surface })
make("UIPadding", {
	PaddingLeft = UDim.new(0, 11),
	PaddingRight = UDim.new(0, 11),
	PaddingTop = UDim.new(0, 6),
	PaddingBottom = UDim.new(0, 6),
	Parent = surface,
})
list(surface, Enum.FillDirection.Horizontal, 8, { VerticalAlignment = Enum.VerticalAlignment.Center })

type Vital = { value: TextLabel, meter: Frame, fill: Frame, spark: Frame, bars: { Frame } }

local function divider(order: number)
	make("Frame", {
		Name = `Divider{order}`,
		LayoutOrder = order,
		Size = UDim2.new(0, 1, 0, 22),
		BackgroundColor3 = WHITE,
		BackgroundTransparency = 0.65,
		Parent = surface,
	})
end

local function vital(name: string, order: number, caption: string): Vital
	local column = make("Frame", {
		Name = name,
		LayoutOrder = order,
		Size = UDim2.new(0, 0, 1, 0),
		BackgroundTransparency = 1,
		Parent = surface,
	})
	make("UIFlexItem", { FlexMode = Enum.UIFlexMode.Fill, Parent = column })
	list(column, Enum.FillDirection.Vertical, 3)

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
		FontFace = CAPS,
		TextSize = 9,
		TextColor3 = WHITE,
		TextTransparency = 0.15,
		TextXAlignment = Enum.TextXAlignment.Left,
		BackgroundTransparency = 1,
		Size = UDim2.fromScale(1, 1),
		Parent = head,
	})
	local value = make("TextLabel", {
		Name = "Value",
		FontFace = MONO,
		TextSize = 12,
		TextColor3 = WHITE,
		TextXAlignment = Enum.TextXAlignment.Right,
		BackgroundTransparency = 1,
		Size = UDim2.fromScale(1, 1),
		Parent = head,
	})
	make("UIStroke", { Thickness = 1, Color = INK, Transparency = 0.6, Parent = value })

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
		AnchorPoint = Vector2.new(0, 0.5),
		Position = UDim2.fromScale(0, 0.5),
		Size = UDim2.new(1, 0, 0, 5),
		BackgroundColor3 = WHITE,
		BackgroundTransparency = 0.75,
		Parent = body,
	})
	make("UICorner", { CornerRadius = UDim.new(0, 3), Parent = meter })
	local fill = make("Frame", {
		Name = "Fill",
		Size = UDim2.fromScale(0, 1),
		BackgroundColor3 = COOL,
		Parent = meter,
	})
	make("UICorner", { CornerRadius = UDim.new(0, 3), Parent = fill })

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
		bars[i] = make("Frame", {
			Name = string.format("Bar%02d", i),
			LayoutOrder = i,
			Size = UDim2.new(0, 1, 0, 1),
			BackgroundColor3 = WHITE,
			BackgroundTransparency = 1,
			Parent = spark,
		})
	end
	return { value = value, meter = meter, fill = fill, spark = spark, bars = bars }
end

local cpu = vital("Cpu", 1, "CPU")
divider(2)
local ram = vital("Memory", 3, "RAM")
divider(4)
local net = vital("Network", 5, "NET")
local vitals = { cpu, ram, net }

local function show()
	for _, v in vitals do
		v.meter.Visible = not graphs
		v.spark.Visible = graphs
	end
end

--- The last `SAMPLES` of `series` against `scale`, newest on the right.
local function spark(v: Vital, series: { number }, scale: number, tint: ((number) -> Color3)?)
	local offset = SAMPLES - #series
	for i, bar in v.bars do
		local value = series[i + (#series - SAMPLES)]
		if i <= offset or value == nil then
			bar.BackgroundTransparency = 1
		else
			bar.Size = UDim2.new(0, 1, math.clamp(value / scale, 0, 1), 1)
			bar.BackgroundTransparency = 0.05 + 0.55 * (1 - i / SAMPLES)
			bar.BackgroundColor3 = if tint then tint(value) else WHITE
		end
	end
end

local function update()
	local c = System.Cpu()
	cpu.value.Text = string.format("%d%%", math.floor(c.usage + 0.5))
	cpu.fill.Size = UDim2.fromScale(c.usage / 100, 1)
	cpu.fill.BackgroundColor3 = heat(c.usage)
	spark(cpu, System.History("cpu"), 100, heat)

	local m = System.Memory()
	local used = if m.total > 0 then m.used / m.total * 100 else 0
	ram.value.Text = string.format("%d%%", math.floor(used + 0.5))
	ram.fill.Size = UDim2.fromScale(used / 100, 1)
	ram.fill.BackgroundColor3 = heat(used)
	spark(ram, System.History("memory"), 100, heat)

	local down = System.History("down")
	local peak = 16 * 1024
	for _, v in down do
		peak = math.max(peak, v)
	end
	local n = System.Network()
	net.value.Text = compact(n.down)
	net.fill.Size = UDim2.fromScale(math.clamp(n.down / peak, 0, 1), 1)
	net.fill.BackgroundColor3 = Color3.fromRGB(150, 245, 215)
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

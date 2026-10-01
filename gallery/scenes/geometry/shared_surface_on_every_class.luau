--[[
	The properties every `GuiObject` shares, varied on each class that can be
	made, so the differential pass reports them per class rather than crediting
	a `TextBox` with what a `Frame` did.

	One cell per class. In each, the class under test (the subject) is a blue
	box clipping an orange strip that reaches past its right edge, under a grey
	box drawn above it by ZIndex. Below it, two rows place the class beside a
	plain frame: one row sorts by LayoutOrder, the other by Name.

	Every variant changes one property on one class and should move pixels.
	A `ScrollingFrame` clips whatever `ClipsDescendants` says, so it is not
	varied there.
]]
local CLASSES = {
	"Frame",
	"TextLabel",
	"TextButton",
	"TextBox",
	"ImageLabel",
	"ImageButton",
	"ScrollingFrame",
	"CanvasGroup",
	"InputActionLabel",
}

local function colour(r, g, b)
	return { "Color3", r, g, b }
end

local function box(x, y, w, h)
	return { "UDim2", 0, x, 0, y }, { "UDim2", 0, w, 0, h }
end

local cells = {}
local variants = {}

for i, class in ipairs(CLASSES) do
	local column = (i - 1) % 3
	local row = (i - 1) // 3
	local subjectPosition, subjectSize = box(10, 10, 60, 30)
	local spillPosition, spillSize = box(40, 2, 50, 8)
	local overPosition, overSize = box(50, 20, 40, 30)

	local subject = {
		class = class,
		name = class .. "Subject",
		props = {
			Position = subjectPosition,
			Size = subjectSize,
			BackgroundColor3 = colour(0.30, 0.62, 0.94),
			BackgroundTransparency = 0,
			BorderSizePixel = 0,
			ZIndex = 1,
		},
		children = {
			{
				class = "Frame",
				name = class .. "Spill",
				props = {
					Position = spillPosition,
					Size = spillSize,
					BackgroundColor3 = colour(0.98, 0.55, 0.20),
					BorderSizePixel = 0,
				},
			},
		},
	}
	if class ~= "ScrollingFrame" then
		subject.props.ClipsDescendants = true
	end

	local function rowOf(name, sortOrder, first, second)
		local position, size = box(10, 0, 180, 30)
		return {
			class = "Frame",
			name = name,
			props = {
				Position = position,
				Size = size,
				BackgroundTransparency = 1,
				BorderSizePixel = 0,
			},
			children = {
				{
					class = "UIListLayout",
					name = name .. "List",
					props = {
						FillDirection = { "Enum", "FillDirection", "Horizontal" },
						SortOrder = { "Enum", "SortOrder", sortOrder },
						Padding = { "UDim", 0, 4 },
					},
				},
				first,
				second,
			},
		}
	end

	local ordered = rowOf("ByOrder" .. class, "LayoutOrder", {
		class = class,
		name = class .. "Ordered",
		props = {
			Size = { "UDim2", 0, 40, 0, 30 },
			BackgroundColor3 = colour(0.55, 0.85, 0.55),
			BorderSizePixel = 0,
			LayoutOrder = 1,
		},
	}, {
		class = "Frame",
		name = "OrderPeer" .. class,
		props = {
			Size = { "UDim2", 0, 40, 0, 30 },
			BackgroundColor3 = colour(0.85, 0.45, 0.55),
			BorderSizePixel = 0,
			LayoutOrder = 2,
		},
	})
	ordered.props.Position = { "UDim2", 0, 10, 0, 60 }

	local named = rowOf("ByName" .. class, "Name", {
		class = class,
		name = "A" .. class,
		props = {
			Size = { "UDim2", 0, 40, 0, 30 },
			BackgroundColor3 = colour(0.65, 0.55, 0.95),
			BorderSizePixel = 0,
		},
	}, {
		class = "Frame",
		name = "B" .. class,
		props = {
			Size = { "UDim2", 0, 40, 0, 30 },
			BackgroundColor3 = colour(0.45, 0.80, 0.80),
			BorderSizePixel = 0,
		},
	})
	named.props.Position = { "UDim2", 0, 10, 0, 100 }

	local cellPosition, cellSize = box(10 + column * 210, 10 + row * 145, 200, 135)
	table.insert(cells, {
		class = "Frame",
		name = "Cell" .. class,
		props = {
			Position = cellPosition,
			Size = cellSize,
			BackgroundColor3 = colour(0.16, 0.17, 0.20),
			BorderSizePixel = 0,
		},
		children = {
			subject,
			{
				class = "Frame",
				name = class .. "Over",
				props = {
					Position = overPosition,
					Size = overSize,
					BackgroundColor3 = colour(0.55, 0.56, 0.60),
					BorderSizePixel = 0,
					ZIndex = 2,
				},
			},
			ordered,
			named,
		},
	})

	local s = class .. "Subject"
	for _, v in
		{
			{ s, "Position", { "UDim2", 0, 20, 0, 15 } },
			{ s, "Size", { "UDim2", 0, 40, 0, 20 } },
			{ s, "AnchorPoint", { "Vector2", 0.5, 0.5 } },
			{ s, "Visible", false },
			{ s, "BackgroundColor3", colour(1, 0, 0) },
			{ s, "BackgroundTransparency", 1 },
			{ s, "ZIndex", 3 },
			{ s, "AutomaticSize", { "Enum", "AutomaticSize", "XY" } },
			{ class .. "Ordered", "LayoutOrder", 3 },
			{ "A" .. class, "Name", "C" .. class },
		}
	do
		table.insert(variants, { node = v[1], prop = v[2], value = v[3] })
	end
	if class ~= "ScrollingFrame" then
		table.insert(variants, { node = s, prop = "ClipsDescendants", value = false })
	end
end

return {
	name = "Every class that can be made: the shared GuiObject surface",
	shows = "Nine cells, one per class, each a blue box clipping an orange strip under a grey box, above a LayoutOrder row and a Name row that put the class first.",
	surface = { width = 640, height = 445 },
	variants = variants,
	tree = {
		class = "Frame",
		name = "Panel",
		props = {
			Size = { "UDim2", 1, 0, 1, 0 },
			BackgroundColor3 = colour(0.10, 0.11, 0.13),
			BorderSizePixel = 0,
		},
		children = cells,
	},
}

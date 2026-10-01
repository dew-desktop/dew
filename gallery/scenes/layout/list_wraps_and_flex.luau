--!strict
--[[
	A horizontal run of three chips that does not quite fit its row, and a
	vertical stack of three bars with room to spare. Every variant changes one
	`UIListLayout` property and should move pixels: wrapping the overflowing chip
	onto a second line, flexing the chips to fill or share the row, centring them
	across their line, sorting them by name instead of `LayoutOrder`, resolving
	`Padding` from a scale, and pushing the stack down its own main axis.
]]
return {
	name = "UIListLayout: wrapping, flex and line alignment",
	shows = "Chips 80, 90 and 70 wide overflow a 240 wide row in LayoutOrder (Cedar, Birch, Ash), top-aligned in their line; three bars stack at the top of a taller box.",
	surface = { width = 260, height = 220 },

	variants = {
		{ node = "Flow", prop = "Wraps", value = true },
		{ node = "Flow", prop = "HorizontalFlex", value = { "Enum", "UIFlexAlignment", "Fill" } },
		{ node = "Flow", prop = "VerticalFlex", value = { "Enum", "UIFlexAlignment", "Fill" } },
		{ node = "Flow", prop = "ItemLineAlignment", value = { "Enum", "ItemLineAlignment", "Center" } },
		{ node = "Flow", prop = "SortOrder", value = { "Enum", "SortOrder", "Name" } },
		{ node = "Flow", prop = "Padding", value = { "UDim", 0.05, 6 } },
		{ node = "Stack", prop = "VerticalAlignment", value = { "Enum", "VerticalAlignment", "Bottom" } },
		{ node = "Stack", prop = "VerticalFlex", value = { "Enum", "UIFlexAlignment", "SpaceEvenly" } },
	},

	tree = {
		class = "Frame",
		name = "Panel",
		props = {
			Size = { "UDim2", 1, 0, 1, 0 },
			BackgroundColor3 = { "Color3", 0.13, 0.14, 0.17 },
			BorderSizePixel = 0,
		},
		children = {
			{
				class = "Frame",
				name = "Row",
				props = {
					Position = { "UDim2", 0, 10, 0, 10 },
					Size = { "UDim2", 0, 240, 0, 96 },
					BackgroundColor3 = { "Color3", 0.19, 0.2, 0.24 },
					BorderSizePixel = 0,
				},
				children = {
					{
						class = "UIListLayout",
						name = "Flow",
						props = {
							FillDirection = { "Enum", "FillDirection", "Horizontal" },
							SortOrder = { "Enum", "SortOrder", "LayoutOrder" },
							Padding = { "UDim", 0, 6 },
							Wraps = false,
							HorizontalFlex = { "Enum", "UIFlexAlignment", "None" },
							VerticalFlex = { "Enum", "UIFlexAlignment", "None" },
							ItemLineAlignment = { "Enum", "ItemLineAlignment", "Start" },
						},
					},
					{
						class = "Frame",
						name = "Ash",
						props = {
							Size = { "UDim2", 0, 80, 0, 24 },
							BackgroundColor3 = { "Color3", 0.30, 0.62, 0.94 },
							BorderSizePixel = 0,
							LayoutOrder = 3,
						},
					},
					{
						class = "Frame",
						name = "Birch",
						props = {
							Size = { "UDim2", 0, 90, 0, 36 },
							BackgroundColor3 = { "Color3", 0.98, 0.71, 0.25 },
							BorderSizePixel = 0,
							LayoutOrder = 2,
						},
					},
					{
						class = "Frame",
						name = "Cedar",
						props = {
							Size = { "UDim2", 0, 70, 0, 28 },
							BackgroundColor3 = { "Color3", 0.55, 0.85, 0.55 },
							BorderSizePixel = 0,
							LayoutOrder = 1,
						},
					},
				},
			},
			{
				class = "Frame",
				name = "Column",
				props = {
					Position = { "UDim2", 0, 10, 0, 116 },
					Size = { "UDim2", 0, 240, 0, 94 },
					BackgroundColor3 = { "Color3", 0.19, 0.2, 0.24 },
					BorderSizePixel = 0,
				},
				children = {
					{
						class = "UIListLayout",
						name = "Stack",
						props = {
							SortOrder = { "Enum", "SortOrder", "LayoutOrder" },
							Padding = { "UDim", 0, 4 },
							VerticalAlignment = { "Enum", "VerticalAlignment", "Top" },
							VerticalFlex = { "Enum", "UIFlexAlignment", "None" },
						},
					},
					{
						class = "Frame",
						name = "Bar1",
						props = {
							Size = { "UDim2", 0, 200, 0, 20 },
							BackgroundColor3 = { "Color3", 0.85, 0.45, 0.55 },
							BorderSizePixel = 0,
							LayoutOrder = 1,
						},
					},
					{
						class = "Frame",
						name = "Bar2",
						props = {
							Size = { "UDim2", 0, 150, 0, 20 },
							BackgroundColor3 = { "Color3", 0.65, 0.55, 0.95 },
							BorderSizePixel = 0,
							LayoutOrder = 2,
						},
					},
					{
						class = "Frame",
						name = "Bar3",
						props = {
							Size = { "UDim2", 0, 100, 0, 20 },
							BackgroundColor3 = { "Color3", 0.45, 0.8, 0.8 },
							BorderSizePixel = 0,
							LayoutOrder = 3,
						},
					},
				},
			},
		},
	},
}

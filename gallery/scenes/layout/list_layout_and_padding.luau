--!strict
--[[
	The parent that rearranges its children. `UIListLayout` assigns each child a
	slot and `UIPadding` insets the whole run, which together are the properties
	most UI actually uses and which no mod in the repository shows in isolation.
]]
return {
	name = "UIListLayout and UIPadding",
	shows = "UIListLayout stacking three rows in LayoutOrder with 8px between them, inset on all four sides by UIPadding.",
	surface = { width = 200, height = 170 },

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
				class = "UIPadding",
				name = "Inset",
				props = {
					PaddingTop = { "UDim", 0, 14 },
					PaddingBottom = { "UDim", 0, 14 },
					PaddingLeft = { "UDim", 0, 18 },
					PaddingRight = { "UDim", 0, 18 },
				},
			},
			{
				class = "UIListLayout",
				name = "Stack",
				props = {
					Padding = { "UDim", 0, 8 },
					FillDirection = { "Enum", "FillDirection", "Vertical" },
					HorizontalAlignment = { "Enum", "HorizontalAlignment", "Center" },
					SortOrder = { "Enum", "SortOrder", "LayoutOrder" },
				},
			},
			{
				class = "Frame",
				name = "Row1",
				props = {
					Size = { "UDim2", 1, 0, 0, 32 },
					BackgroundColor3 = { "Color3", 0.30, 0.62, 0.94 },
					BorderSizePixel = 0,
					LayoutOrder = 1,
				},
			},
			{
				class = "Frame",
				name = "Row2",
				props = {
					Size = { "UDim2", 1, 0, 0, 32 },
					BackgroundColor3 = { "Color3", 0.98, 0.71, 0.25 },
					BorderSizePixel = 0,
					LayoutOrder = 2,
				},
			},
			{
				class = "Frame",
				name = "Row3",
				props = {
					Size = { "UDim2", 1, 0, 0, 32 },
					BackgroundColor3 = { "Color3", 0.55, 0.85, 0.55 },
					BorderSizePixel = 0,
					LayoutOrder = 3,
				},
			},
		},
	},
}

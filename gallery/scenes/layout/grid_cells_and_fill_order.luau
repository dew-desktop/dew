--!strict
--[[
	Seven coloured tiles in one `UIGridLayout`, four to a row, and the sixth
	carrying a `UIAspectRatioConstraint` of 1. Every variant changes one property
	and should move pixels: a new cell size, wider gaps between cells, filling
	down before across, capping a row at three cells, filling from the bottom
	right corner, centring the block, and an aspect ratio of 2 that leaves the
	sixth tile half as tall and centred in its cell.
]]
return {
	name = "UIGridLayout: cells, fill order and an aspect ratio in a cell",
	shows = "Tiles 1 to 7 fill 50 pixel cells with 6 pixel gaps, four to a row from the top left; tile 6 is square because of its aspect ratio of 1.",
	surface = { width = 280, height = 220 },

	variants = {
		{ node = "Grid", prop = "CellSize", value = { "UDim2", 0, 40, 0, 60 } },
		{ node = "Grid", prop = "CellPadding", value = { "UDim2", 0, 16, 0, 16 } },
		{ node = "Grid", prop = "FillDirection", value = { "Enum", "FillDirection", "Vertical" } },
		{ node = "Grid", prop = "FillDirectionMaxCells", value = 3 },
		{ node = "Grid", prop = "StartCorner", value = { "Enum", "StartCorner", "BottomRight" } },
		{ node = "Grid", prop = "HorizontalAlignment", value = { "Enum", "HorizontalAlignment", "Center" } },
		{ node = "Shape", prop = "AspectRatio", value = 2 },
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
				name = "Tray",
				props = {
					Position = { "UDim2", 0, 10, 0, 10 },
					Size = { "UDim2", 0, 260, 0, 200 },
					BackgroundColor3 = { "Color3", 0.19, 0.2, 0.24 },
					BorderSizePixel = 0,
				},
				children = {
					{
						class = "UIGridLayout",
						name = "Grid",
						props = {
							SortOrder = { "Enum", "SortOrder", "LayoutOrder" },
							CellSize = { "UDim2", 0, 50, 0, 50 },
							CellPadding = { "UDim2", 0, 6, 0, 6 },
							FillDirection = { "Enum", "FillDirection", "Horizontal" },
							FillDirectionMaxCells = 0,
							StartCorner = { "Enum", "StartCorner", "TopLeft" },
						},
					},
					{
						class = "Frame",
						name = "Tile1",
						props = { BackgroundColor3 = { "Color3", 0.30, 0.62, 0.94 }, BorderSizePixel = 0, LayoutOrder = 1 },
					},
					{
						class = "Frame",
						name = "Tile2",
						props = { BackgroundColor3 = { "Color3", 0.98, 0.71, 0.25 }, BorderSizePixel = 0, LayoutOrder = 2 },
					},
					{
						class = "Frame",
						name = "Tile3",
						props = { BackgroundColor3 = { "Color3", 0.55, 0.85, 0.55 }, BorderSizePixel = 0, LayoutOrder = 3 },
					},
					{
						class = "Frame",
						name = "Tile4",
						props = { BackgroundColor3 = { "Color3", 0.85, 0.45, 0.55 }, BorderSizePixel = 0, LayoutOrder = 4 },
					},
					{
						class = "Frame",
						name = "Tile5",
						props = { BackgroundColor3 = { "Color3", 0.65, 0.55, 0.95 }, BorderSizePixel = 0, LayoutOrder = 5 },
					},
					{
						class = "Frame",
						name = "Tile6",
						props = {
							BackgroundColor3 = { "Color3", 0.45, 0.80, 0.80 },
							BorderSizePixel = 0,
							LayoutOrder = 6,
						},
						children = {
							{ class = "UIAspectRatioConstraint", name = "Shape", props = { AspectRatio = 1 } },
						},
					},
					{
						class = "Frame",
						name = "Tile7",
						props = { BackgroundColor3 = { "Color3", 0.95, 0.60, 0.40 }, BorderSizePixel = 0, LayoutOrder = 7 },
					},
				},
			},
		},
	},
}

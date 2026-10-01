--!strict
--[[
	Three rows, each a horizontal UIListLayout whose children carry a
	UIFlexItem, and two of those children carry a UISizeConstraint. Every
	variant changes one property of a flex item or a constraint and should move
	pixels: the field stops growing, two blocks split their row by a new ratio,
	one block sits differently across its line, the overflow is shared by a new
	ratio, and the bounds stop a growing or shrinking child early.
]]
return {
	name = "UIFlexItem and UISizeConstraint: per-child flex and bounds",
	shows = "Top: a Grow field fills the row beside a fixed button. Middle: two Custom blocks split the spare width one to one, the short one at the top of its line. Bottom: two Shrink blocks too wide for the row give back the overflow in proportion to their widths.",
	surface = { width = 260, height = 170 },

	variants = {
		{ node = "FieldFlex", prop = "FlexMode", value = { "Enum", "UIFlexMode", "None" } },
		{ node = "FieldLimit", prop = "MaxSize", value = { "Vector2", 110, 1000 } },
		{ node = "RightFlex", prop = "GrowRatio", value = 3 },
		{ node = "LeftFlex", prop = "ItemLineAlignment", value = { "Enum", "ItemLineAlignment", "End" } },
		{ node = "NarrowFlex", prop = "ShrinkRatio", value = 4 },
		{ node = "WideLimit", prop = "MinSize", value = { "Vector2", 140, 0 } },
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
				name = "GrowRow",
				props = {
					Position = { "UDim2", 0, 10, 0, 10 },
					Size = { "UDim2", 0, 240, 0, 40 },
					BackgroundColor3 = { "Color3", 0.19, 0.2, 0.24 },
					BorderSizePixel = 0,
				},
				children = {
					{
						class = "UIListLayout",
						name = "GrowList",
						props = {
							FillDirection = { "Enum", "FillDirection", "Horizontal" },
							SortOrder = { "Enum", "SortOrder", "LayoutOrder" },
							VerticalAlignment = { "Enum", "VerticalAlignment", "Center" },
							Padding = { "UDim", 0, 6 },
						},
					},
					{
						class = "Frame",
						name = "Field",
						props = {
							Size = { "UDim2", 0, 60, 0, 28 },
							BackgroundColor3 = { "Color3", 0.30, 0.62, 0.94 },
							BorderSizePixel = 0,
							LayoutOrder = 1,
						},
						children = {
							{
								class = "UIFlexItem",
								name = "FieldFlex",
								props = { FlexMode = { "Enum", "UIFlexMode", "Grow" } },
							},
							{ class = "UISizeConstraint", name = "FieldLimit", props = {} },
						},
					},
					{
						class = "Frame",
						name = "Send",
						props = {
							Size = { "UDim2", 0, 50, 0, 28 },
							BackgroundColor3 = { "Color3", 0.98, 0.71, 0.25 },
							BorderSizePixel = 0,
							LayoutOrder = 2,
						},
					},
				},
			},
			{
				class = "Frame",
				name = "RatioRow",
				props = {
					Position = { "UDim2", 0, 10, 0, 60 },
					Size = { "UDim2", 0, 240, 0, 44 },
					BackgroundColor3 = { "Color3", 0.19, 0.2, 0.24 },
					BorderSizePixel = 0,
				},
				children = {
					{
						class = "UIListLayout",
						name = "RatioList",
						props = {
							FillDirection = { "Enum", "FillDirection", "Horizontal" },
							SortOrder = { "Enum", "SortOrder", "LayoutOrder" },
							ItemLineAlignment = { "Enum", "ItemLineAlignment", "Start" },
							Padding = { "UDim", 0, 6 },
						},
					},
					{
						class = "Frame",
						name = "Left",
						props = {
							Size = { "UDim2", 0, 40, 0, 16 },
							BackgroundColor3 = { "Color3", 0.55, 0.85, 0.55 },
							BorderSizePixel = 0,
							LayoutOrder = 1,
						},
						children = {
							{
								class = "UIFlexItem",
								name = "LeftFlex",
								props = {
									FlexMode = { "Enum", "UIFlexMode", "Custom" },
									GrowRatio = 1,
								},
							},
						},
					},
					{
						class = "Frame",
						name = "Right",
						props = {
							Size = { "UDim2", 0, 40, 0, 40 },
							BackgroundColor3 = { "Color3", 0.85, 0.45, 0.55 },
							BorderSizePixel = 0,
							LayoutOrder = 2,
						},
						children = {
							{
								class = "UIFlexItem",
								name = "RightFlex",
								props = {
									FlexMode = { "Enum", "UIFlexMode", "Custom" },
									GrowRatio = 1,
								},
							},
						},
					},
				},
			},
			{
				class = "Frame",
				name = "ShrinkRow",
				props = {
					Position = { "UDim2", 0, 10, 0, 114 },
					Size = { "UDim2", 0, 240, 0, 44 },
					BackgroundColor3 = { "Color3", 0.19, 0.2, 0.24 },
					BorderSizePixel = 0,
				},
				children = {
					{
						class = "UIListLayout",
						name = "ShrinkList",
						props = {
							FillDirection = { "Enum", "FillDirection", "Horizontal" },
							SortOrder = { "Enum", "SortOrder", "LayoutOrder" },
							VerticalAlignment = { "Enum", "VerticalAlignment", "Center" },
						},
					},
					{
						class = "Frame",
						name = "Wide",
						props = {
							Size = { "UDim2", 0, 160, 0, 30 },
							BackgroundColor3 = { "Color3", 0.65, 0.55, 0.95 },
							BorderSizePixel = 0,
							LayoutOrder = 1,
						},
						children = {
							{
								class = "UIFlexItem",
								name = "WideFlex",
								props = {
									FlexMode = { "Enum", "UIFlexMode", "Custom" },
									ShrinkRatio = 1,
								},
							},
							{ class = "UISizeConstraint", name = "WideLimit", props = {} },
						},
					},
					{
						class = "Frame",
						name = "Narrow",
						props = {
							Size = { "UDim2", 0, 120, 0, 30 },
							BackgroundColor3 = { "Color3", 0.45, 0.8, 0.8 },
							BorderSizePixel = 0,
							LayoutOrder = 2,
						},
						children = {
							{
								class = "UIFlexItem",
								name = "NarrowFlex",
								props = {
									FlexMode = { "Enum", "UIFlexMode", "Custom" },
									ShrinkRatio = 1,
								},
							},
						},
					},
				},
			},
		},
	},
}

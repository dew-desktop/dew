--!strict
--[[
	A `ScrollingFrame` is a container that offers a scrollable canvas.
	Descendants are clipped to the frame's own bounds and shifted by
	the `CanvasPosition` scroll offset. `AutomaticCanvasSize` allows the
	canvas to expand dynamically to fit overflowing content.
]]
return {
	name = "ScrollingFrame: canvas, scroll position and clipping",
	shows = "A ScrollingFrame with four colored cards on an extended canvas, shifted by CanvasPosition and clipped cleanly to the frame's rounded boundaries.",
	surface = { width = 280, height = 200 },

	variants = {
		{ node = "Scroll", prop = "CanvasPosition", value = { "Vector2", 0, 20 } },
		{ node = "Scroll", prop = "CanvasSize", value = { "UDim2", 0, 180, 0, 250 } },
		{ node = "Scroll", prop = "AutomaticCanvasSize", value = { "Enum", "AutomaticSize", "None" } },
		{ node = "Card3", prop = "BackgroundColor3", value = { "Color3", 1, 0, 0 } },
	},

	tree = {
		class = "Frame",
		name = "Backdrop",
		props = {
			Size = { "UDim2", 1, 0, 1, 0 },
			BackgroundColor3 = { "Color3", 0.11, 0.12, 0.15 },
			BorderSizePixel = 0,
		},
		children = {
			{
				class = "ScrollingFrame",
				name = "Scroll",
				props = {
					Position = { "UDim2", 0, 20, 0, 20 },
					Size = { "UDim2", 0, 240, 0, 160 },
					CanvasSize = { "UDim2", 1, 0, 0, 250 },
					CanvasPosition = { "Vector2", 0, 110 },
					AutomaticCanvasSize = { "Enum", "AutomaticSize", "Y" },
					ScrollingDirection = { "Enum", "ScrollingDirection", "XY" },
					ScrollingEnabled = true,
					ClipsDescendants = true,
					BackgroundColor3 = { "Color3", 0.18, 0.19, 0.24 },
					BorderSizePixel = 0,
				},
				children = {
					{
						class = "UICorner",
						name = "FrameCorner",
						props = { CornerRadius = { "UDim", 0, 8 } },
					},
					{
						class = "Frame",
						name = "Card1",
						props = {
							Position = { "UDim2", 0, 15, 0, 15 },
							Size = { "UDim2", 1, -30, 0, 55 },
							BackgroundColor3 = { "Color3", 0.30, 0.62, 0.94 },
							BorderSizePixel = 0,
						},
						children = {
							{
								class = "UICorner",
								name = "Card1Corner",
								props = { CornerRadius = { "UDim", 0, 6 } },
							},
						},
					},
					{
						class = "Frame",
						name = "Card2",
						props = {
							Position = { "UDim2", 0, 15, 0, 85 },
							Size = { "UDim2", 1, -30, 0, 55 },
							BackgroundColor3 = { "Color3", 0.98, 0.71, 0.25 },
							BorderSizePixel = 0,
						},
						children = {
							{
								class = "UICorner",
								name = "Card2Corner",
								props = { CornerRadius = { "UDim", 0, 6 } },
							},
						},
					},
					{
						class = "Frame",
						name = "Card3",
						props = {
							Position = { "UDim2", 0, 15, 0, 155 },
							Size = { "UDim2", 1, -30, 0, 55 },
							BackgroundColor3 = { "Color3", 0.55, 0.85, 0.55 },
							BorderSizePixel = 0,
						},
						children = {
							{
								class = "UICorner",
								name = "Card3Corner",
								props = { CornerRadius = { "UDim", 0, 6 } },
							},
						},
					},
					{
						class = "Frame",
						name = "Card4",
						props = {
							Position = { "UDim2", 0, 15, 0, 225 },
							Size = { "UDim2", 1, -30, 0, 55 },
							BackgroundColor3 = { "Color3", 0.85, 0.45, 0.65 },
							BorderSizePixel = 0,
						},
						children = {
							{
								class = "UICorner",
								name = "Card4Corner",
								props = { CornerRadius = { "UDim", 0, 6 } },
							},
						},
					},
				},
			},
		},
	},
}

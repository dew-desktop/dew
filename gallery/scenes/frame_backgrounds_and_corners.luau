--!strict
--[[
	A `Frame` is the smallest thing the host draws, and this is the set of
	properties that decide what a rectangle looks like before any text or image
	is involved: where it sits, how big it is, what colour it is, how solid, and
	whether its corners are cut.

	NO EXPECTATION BLOCK, ON PURPOSE. This is a gallery scene, not a conformance
	case: it asserts nothing about what Roblox would do with the same tree. It
	exists so that these properties are seen going through the host and out to
	pixels. See ADR-005.
]]
return {
	name = "Frame: backgrounds, borders and corners",
	surface = { width = 260, height = 140 },

	tree = {
		class = "Frame",
		name = "Backdrop",
		props = {
			Size = { "UDim2", 1, 0, 1, 0 },
			BackgroundColor3 = { "Color3", 0.12, 0.13, 0.16 },
			BorderSizePixel = 0,
		},
		children = {
			{
				class = "Frame",
				name = "Solid",
				props = {
					Position = { "UDim2", 0, 16, 0, 16 },
					Size = { "UDim2", 0, 100, 0, 48 },
					BackgroundColor3 = { "Color3", 0.30, 0.62, 0.94 },
					BorderSizePixel = 0,
					ZIndex = 1,
					Visible = true,
				},
				children = {
					{ class = "UICorner", name = "Round", props = { CornerRadius = { "UDim", 0, 10 } } },
				},
			},
			{
				class = "Frame",
				name = "Translucent",
				props = {
					Position = { "UDim2", 0, 76, 0, 46 },
					Size = { "UDim2", 0, 100, 0, 48 },
					BackgroundColor3 = { "Color3", 0.98, 0.71, 0.25 },
					BackgroundTransparency = 0.35,
					BorderSizePixel = 0,
					-- Deliberately above the solid block, so the overlap shows
					-- both the transparency and the paint order.
					ZIndex = 2,
				},
				children = {
					{ class = "UICorner", name = "Round", props = { CornerRadius = { "UDim", 0, 10 } } },
				},
			},
			{
				class = "Frame",
				name = "AnchoredRight",
				props = {
					AnchorPoint = { "Vector2", 1, 1 },
					Position = { "UDim2", 1, -16, 1, -16 },
					Size = { "UDim2", 0, 56, 0, 56 },
					BackgroundColor3 = { "Color3", 0.55, 0.85, 0.55 },
					BorderSizePixel = 0,
				},
				children = {
					{ class = "UICorner", name = "Round", props = { CornerRadius = { "UDim", 1, 0 } } },
				},
			},
		},
	},
}

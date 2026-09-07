--!strict
--[[
	The two modifiers that change how a shape is filled and outlined, rather
	than where it sits: `UIGradient` with a real `ColorSequence`, and `UIStroke`
	with a colour, a thickness and a transparency.

	The sequence is deliberately not white-to-black. Milestone 3 shipped a
	default white-to-black ramp for gradients with no sequence set, and milestone
	4 removed it; a scene using those two colours would look identical whether
	the sequence reached the display list or not, which is the whole failure
	this gallery is meant to make visible.
]]
return {
	name = "UIGradient and UIStroke",
	shows = "A red-to-blue ColorSequence gradient rotated 35 degrees, beside a frame outlined by a green UIStroke. Not white-to-black, so a fallback ramp could not fake it.",
	surface = { width = 280, height = 130 },

	tree = {
		class = "Frame",
		name = "Backdrop",
		props = {
			Size = { "UDim2", 1, 0, 1, 0 },
			BackgroundColor3 = { "Color3", 0.09, 0.09, 0.11 },
			BorderSizePixel = 0,
		},
		children = {
			{
				class = "Frame",
				name = "Gradient",
				props = {
					Position = { "UDim2", 0, 16, 0, 20 },
					Size = { "UDim2", 0, 110, 0, 90 },
					BackgroundColor3 = { "Color3", 1, 1, 1 },
					BorderSizePixel = 0,
				},
				children = {
					{ class = "UICorner", name = "Round", props = { CornerRadius = { "UDim", 0, 8 } } },
					{
						class = "UIGradient",
						name = "Ramp",
						props = {
							Color = { "ColorSequence", { "Color3", 0.95, 0.25, 0.35 }, { "Color3", 0.20, 0.35, 0.95 } },
							Rotation = 35,
							Enabled = true,
						},
					},
				},
			},
			{
				class = "Frame",
				name = "Stroked",
				props = {
					Position = { "UDim2", 0, 152, 0, 20 },
					Size = { "UDim2", 0, 110, 0, 90 },
					BackgroundColor3 = { "Color3", 0.16, 0.17, 0.20 },
					BorderSizePixel = 0,
				},
				children = {
					{ class = "UICorner", name = "Round", props = { CornerRadius = { "UDim", 0, 8 } } },
					{
						class = "UIStroke",
						name = "Edge",
						props = {
							Color = { "Color3", 0.35, 0.95, 0.70 },
							Thickness = 3,
							Transparency = 0.1,
						},
					},
				},
			},
		},
	},
}

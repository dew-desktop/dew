--!strict
--[[
	Two frames of 100 by 60, each shaped by a `UIAspectRatioConstraint` of 2.
	The left one scales with its parent along its dominant axis, Width, so it is
	100 by 50. The right one fits within its size, which ignores the dominant
	axis, so it is 100 by 50 too.

	The variants are the two properties that tell the modes apart: making the
	left one's dominant axis Height makes it 120 by 60, and making the right one
	scale with its parent along its own dominant axis, Height, does the same.
]]
return {
	name = "UIAspectRatioConstraint: scale with the parent, or fit within",
	shows = "Two blue frames 100 by 50, one shaped by ScaleWithParentSize along Width and one by FitWithinMaxSize with Height as its unused dominant axis.",
	surface = { width = 300, height = 100 },

	variants = {
		{ node = "ScaleRatio", prop = "DominantAxis", value = { "Enum", "DominantAxis", "Height" } },
		{ node = "FitRatio", prop = "AspectType", value = { "Enum", "AspectType", "ScaleWithParentSize" } },
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
				name = "Scaled",
				props = {
					Position = { "UDim2", 0, 10, 0, 10 },
					Size = { "UDim2", 0, 100, 0, 60 },
					BackgroundColor3 = { "Color3", 0.30, 0.62, 0.94 },
					BorderSizePixel = 0,
				},
				children = {
					{
						class = "UIAspectRatioConstraint",
						name = "ScaleRatio",
						props = {
							AspectRatio = 2,
							AspectType = { "Enum", "AspectType", "ScaleWithParentSize" },
							DominantAxis = { "Enum", "DominantAxis", "Width" },
						},
					},
				},
			},
			{
				class = "Frame",
				name = "Fitted",
				props = {
					Position = { "UDim2", 0, 160, 0, 10 },
					Size = { "UDim2", 0, 100, 0, 60 },
					BackgroundColor3 = { "Color3", 0.30, 0.62, 0.94 },
					BorderSizePixel = 0,
				},
				children = {
					{
						class = "UIAspectRatioConstraint",
						name = "FitRatio",
						props = {
							AspectRatio = 2,
							AspectType = { "Enum", "AspectType", "FitWithinMaxSize" },
							DominantAxis = { "Enum", "DominantAxis", "Height" },
						},
					},
				},
			},
		},
	},
}

--!strict
--[[
	`UIStroke.BorderStrokePosition`, on three rounded cards that differ only in it.

	`Outer`, the engine's default, draws the stroke wholly outside the box,
	`Center` straddles the edge, and `Inner` draws it wholly inside. The three
	cards are the same size, so the stroke's outer edge steps inward from left
	to right by half its thickness each time, and its corner keeps the same
	width as its straight sides in all three.

	A gallery scene, not a conformance case. It asserts nothing about the engine.
]]
local function card(name: string, x: number, position: string)
	return {
		class = "Frame",
		name = name,
		props = {
			Position = { "UDim2", 0, x, 0, 24 },
			Size = { "UDim2", 0, 112, 0, 64 },
			BackgroundColor3 = { "Color3", 0.18, 0.19, 0.23 },
			BorderSizePixel = 0,
		},
		children = {
			{
				class = "UICorner",
				name = name .. "Corner",
				props = { CornerRadius = { "UDim", 0, 16 } },
			},
			{
				class = "UIStroke",
				name = name .. "Edge",
				props = {
					BorderStrokePosition = { "Enum", "BorderStrokePosition", position },
					Color = { "Color3", 0.34, 0.64, 0.94 },
					Thickness = 6,
				},
			},
		},
	}
end

return {
	name = "UIStroke: BorderStrokePosition",
	shows = "Three rounded cards of one size whose 6px strokes sit outside, across and inside the box edge.",
	surface = { width = 440, height = 112 },

	variants = {
		{ node = "OuterEdge", prop = "BorderStrokePosition", value = { "Enum", "BorderStrokePosition", "Inner" } },
		{ node = "InnerEdge", prop = "BorderStrokePosition", value = { "Enum", "BorderStrokePosition", "Center" } },
	},

	tree = {
		class = "Frame",
		name = "Backdrop",
		props = {
			Size = { "UDim2", 1, 0, 1, 0 },
			BackgroundColor3 = { "Color3", 0.09, 0.09, 0.11 },
			BorderSizePixel = 0,
		},
		children = {
			card("Outer", 24, "Outer"),
			card("Center", 164, "Center"),
			card("Inner", 304, "Inner"),
		},
	},
}

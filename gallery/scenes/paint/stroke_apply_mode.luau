--!strict
--[[
	`UIStroke.ApplyStrokeMode`, on the two kinds of parent it tells apart.

	`Contextual`, the default, outlines the glyphs of a text object and the
	border of anything else. `Border` outlines the box whatever the parent is.
	The first two labels differ only in that one property, and the frame shows
	`Contextual` falling back to the border.

	A gallery scene, not a conformance case. It asserts nothing about the engine.
]]
local function label(name: string, x: number, mode: string)
	return {
		class = "TextLabel",
		name = name,
		props = {
			Position = { "UDim2", 0, x, 0, 24 },
			Size = { "UDim2", 0, 120, 0, 56 },
			BackgroundColor3 = { "Color3", 0.18, 0.19, 0.23 },
			BorderSizePixel = 0,
			Text = "Stroke",
			TextColor3 = { "Color3", 0.98, 0.96, 0.90 },
			TextSize = 30,
		},
		children = {
			{
				class = "UIStroke",
				name = name .. "Edge",
				props = {
					ApplyStrokeMode = { "Enum", "ApplyStrokeMode", mode },
					Color = { "Color3", 0.95, 0.35, 0.30 },
					Thickness = 2,
				},
			},
		},
	}
end

return {
	name = "UIStroke: ApplyStrokeMode",
	shows = "A TextLabel whose Contextual stroke outlines its glyphs, the same label with Border outlining its box, and a Frame where Contextual outlines the box.",
	surface = { width = 440, height = 104 },

	variants = {
		{ node = "ContextualEdge", prop = "ApplyStrokeMode", value = { "Enum", "ApplyStrokeMode", "Border" } },
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
			label("Contextual", 16, "Contextual"),
			label("Border", 160, "Border"),
			{
				class = "Frame",
				name = "Plain",
				props = {
					Position = { "UDim2", 0, 304, 0, 24 },
					Size = { "UDim2", 0, 120, 0, 56 },
					BackgroundColor3 = { "Color3", 0.18, 0.19, 0.23 },
					BorderSizePixel = 0,
				},
				children = {
					{
						class = "UIStroke",
						name = "PlainEdge",
						props = {
							Color = { "Color3", 0.95, 0.35, 0.30 },
							Thickness = 2,
						},
					},
				},
			},
		},
	},
}

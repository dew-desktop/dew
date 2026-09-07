--!strict
--[[
	Text, and the properties that decide how it is drawn rather than what it
	says. Alignment is the interesting half: the same string in the same box
	lands in nine different places, and nothing else in the repository showed
	that happening.

	A gallery scene, not a conformance case. It asserts nothing about the engine.

	WHAT THIS SCENE CURRENTLY SHOWS, and it is not what it was written to show:
	the `Wrapped` label does NOT wrap. `TextWrapped` grows the element's box in
	layout -- `text_wrapping_grows_height` passes on that -- but the display list
	carries no wrapped flag and `fill_text` does no line breaking, so the painted
	string is a single clipped line.

	Left in deliberately. The gallery's whole job is to show what the host
	actually draws, and a scene edited to hide a gap is a scene that lies. This
	is reported as a finding rather than fixed here; fixing it is not this
	sprint's scope.
]]
return {
	name = "TextLabel: colour, size and alignment",
	shows = "The same font at four alignments and transparencies -- and a TextWrapped label that does NOT wrap, painting one clipped line while its box grows.",
	surface = { width = 300, height = 160 },

	tree = {
		class = "Frame",
		name = "Backdrop",
		props = {
			Size = { "UDim2", 1, 0, 1, 0 },
			BackgroundColor3 = { "Color3", 0.10, 0.11, 0.13 },
			BorderSizePixel = 0,
		},
		children = {
			{
				class = "TextLabel",
				name = "Heading",
				props = {
					Position = { "UDim2", 0, 12, 0, 10 },
					Size = { "UDim2", 1, -24, 0, 34 },
					BackgroundTransparency = 1,
					Text = "The surface demonstrates itself",
					TextColor3 = { "Color3", 0.96, 0.96, 0.98 },
					TextSize = 20,
					TextXAlignment = { "Enum", "TextXAlignment", "Left" },
					TextYAlignment = { "Enum", "TextYAlignment", "Center" },
				},
			},
			{
				class = "TextLabel",
				name = "Centred",
				props = {
					Position = { "UDim2", 0, 12, 0, 54 },
					Size = { "UDim2", 0, 130, 0, 40 },
					BackgroundColor3 = { "Color3", 0.20, 0.22, 0.26 },
					BorderSizePixel = 0,
					Text = "centred",
					TextColor3 = { "Color3", 0.55, 0.85, 1.00 },
					TextSize = 16,
					TextXAlignment = { "Enum", "TextXAlignment", "Center" },
					TextYAlignment = { "Enum", "TextYAlignment", "Center" },
				},
			},
			{
				class = "TextLabel",
				name = "Faded",
				props = {
					Position = { "UDim2", 0, 152, 0, 54 },
					Size = { "UDim2", 0, 130, 0, 40 },
					BackgroundColor3 = { "Color3", 0.20, 0.22, 0.26 },
					BorderSizePixel = 0,
					Text = "faded, right",
					TextColor3 = { "Color3", 1.00, 1.00, 1.00 },
					TextTransparency = 0.45,
					TextSize = 16,
					TextXAlignment = { "Enum", "TextXAlignment", "Right" },
				},
			},
			{
				class = "TextLabel",
				name = "Wrapped",
				props = {
					Position = { "UDim2", 0, 12, 0, 104 },
					Size = { "UDim2", 1, -24, 0, 44 },
					BackgroundTransparency = 1,
					Text = "TextWrapped breaks this sentence across more than one line inside its box.",
					TextColor3 = { "Color3", 0.75, 0.78, 0.82 },
					TextSize = 13,
					TextWrapped = true,
					TextXAlignment = { "Enum", "TextXAlignment", "Left" },
					TextYAlignment = { "Enum", "TextYAlignment", "Top" },
				},
			},
		},
	},
}

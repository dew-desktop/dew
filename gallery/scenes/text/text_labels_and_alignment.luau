--!strict
--[[
	Text, and the properties that decide how it is drawn rather than what it
	says. Alignment is the interesting half: the same string in the same box
	lands in nine different places, and nothing else in the repository showed
	that happening.

	A gallery scene, not a conformance case. It asserts nothing about the engine.

	The `Wrapped` label breaks its sentence across two lines inside its box.
]]
return {
	name = "TextLabel: colour, size and alignment",
	shows = "The same font at four alignments and transparencies, and a TextWrapped label breaking its sentence across two lines.",
	surface = { width = 300, height = 160 },

	variants = {
		{ node = "Heading", prop = "Text", value = "a different string entirely" },
		{ node = "Heading", prop = "TextColor3", value = { "Color3", 1, 0, 0 } },
		{ node = "Heading", prop = "TextSize", value = 32 },
		{ node = "Centred", prop = "TextXAlignment", value = { "Enum", "TextXAlignment", "Left" } },
		{ node = "Centred", prop = "TextYAlignment", value = { "Enum", "TextYAlignment", "Top" } },
		{ node = "Faded", prop = "TextTransparency", value = 0 },
		{ node = "Wrapped", prop = "TextWrapped", value = false },
	},

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

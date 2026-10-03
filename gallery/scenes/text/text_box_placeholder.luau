--[[
	A TextBox with empty Text shows its PlaceholderText, laid out as an
	engine probe lays out its rows so the image can be set beside a Studio
	screenshot of the same rows.

	Each row is a white 260 by 30 TextBox at TextSize 20 in LegacyArial, left
	aligned, with the placeholder "Hamburgefonts" unless stated:

	   1  empty Text
	   2  Text "Typed"
	   3  Text " " (one space)
	   4  empty, TextXAlignment Right
	   5  empty, TextYAlignment Top
	   6  empty, TextTransparency 0.6
	   7  empty, red TextStroke
	   8  empty, PlaceholderColor3 red, TextColor3 blue
	   9  empty, a long placeholder, TextWrapped, TextTruncate AtEnd
	  10  empty, width 0, AutomaticSize X
	  11  empty, RichText, placeholder "<b>Bold</b> hint"

	Only the colour differs from Text: rows 6 and 7 fade and stroke the
	placeholder as they would Text, which the engine is not yet measured to do.
]]
local PH = "Hamburgefonts"

local SETUPS = {
	{},
	{ Text = "Typed" },
	{ Text = " " },
	{ TextXAlignment = { "Enum", "TextXAlignment", "Right" } },
	{ TextYAlignment = { "Enum", "TextYAlignment", "Top" } },
	{ TextTransparency = 0.6 },
	{ TextStrokeTransparency = 0, TextStrokeColor3 = { "Color3", 1, 0, 0 } },
	{ PlaceholderColor3 = { "Color3", 1, 0, 0 }, TextColor3 = { "Color3", 0, 0, 1 } },
	{
		PlaceholderText = "A placeholder far too long to fit on one line here",
		TextWrapped = true,
		TextTruncate = { "Enum", "TextTruncate", "AtEnd" },
	},
	{ Size = { "UDim2", 0, 0, 0, 30 }, AutomaticSize = { "Enum", "AutomaticSize", "X" } },
	{ RichText = true, PlaceholderText = "<b>Bold</b> hint" },
}

local rows = {}
for i, setup in ipairs(SETUPS) do
	local props = {
		Size = { "UDim2", 0, 260, 0, 30 },
		Position = { "UDim2", 0, 20, 0, 10 + (i - 1) * 40 },
		BackgroundColor3 = { "Color3", 1, 1, 1 },
		BorderSizePixel = 0,
		TextColor3 = { "Color3", 0, 0, 0 },
		Font = { "Enum", "Font", "Arial" },
		TextSize = 20,
		TextXAlignment = { "Enum", "TextXAlignment", "Left" },
		ClearTextOnFocus = false,
		Text = "",
		PlaceholderText = PH,
	}
	for key, value in setup do
		props[key] = value
	end
	table.insert(rows, { class = "TextBox", name = "Row" .. i, props = props })
end

return {
	name = "TextBox: the placeholder",
	shows = "Eleven white boxes; every one with empty Text shows Hamburgefonts in grey, except row 8 in red, row 9 cut short with an ellipsis and row 11 reading Bold hint, while rows 2 and 3 show their own text instead.",
	surface = { width = 300, height = 450 },
	variants = {
		{ node = "Row1", prop = "PlaceholderText", value = "Search" },
		{ node = "Row1", prop = "PlaceholderColor3", value = { "Color3", 0, 0.6, 0 } },
	},
	tree = {
		class = "Frame",
		name = "Panel",
		props = {
			Size = { "UDim2", 1, 0, 1, 0 },
			BackgroundColor3 = { "Color3", 0.85, 0.86, 0.88 },
			BorderSizePixel = 0,
		},
		children = rows,
	},
}

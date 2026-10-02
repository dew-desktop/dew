--[[
	The text properties, varied on each class that draws text, so a property
	shown on a `TextLabel` is not credited to a `TextButton` by its name alone.

	One row per class: a dark box holding one line too long to fit, centred and
	unwrapped. An `InputActionLabel` draws its `InputAction` where the others
	draw `Text`, and has no `TextScaled`.
]]
local CLASSES = { "TextLabel", "TextButton", "TextBox", "InputActionLabel" }
local LINE = "Wraps across two lines here"

local rows = {}
local variants = {}

for i, class in ipairs(CLASSES) do
	local name = class .. "Words"
	local props = {
		Position = { "UDim2", 0, 40, 0, 10 + (i - 1) * 50 },
		Size = { "UDim2", 0, 140, 0, 40 },
		BackgroundColor3 = { "Color3", 0.16, 0.17, 0.20 },
		BorderSizePixel = 0,
		TextColor3 = { "Color3", 0.96, 0.96, 0.98 },
		TextSize = 14,
		TextTransparency = 0,
		TextWrapped = false,
		TextXAlignment = { "Enum", "TextXAlignment", "Center" },
		TextYAlignment = { "Enum", "TextYAlignment", "Center" },
	}
	local textProperty = if class == "InputActionLabel" then "InputAction" else "Text"
	props[textProperty] = LINE
	if class ~= "InputActionLabel" then
		props.TextScaled = false
	end
	table.insert(rows, { class = class, name = name, props = props })

	for _, v in
		{
			{ textProperty, "Short" },
			{ "TextColor3", { "Color3", 1, 0, 0 } },
			{ "TextSize", 20 },
			{ "TextTransparency", 1 },
			{ "TextXAlignment", { "Enum", "TextXAlignment", "Left" } },
			{ "TextYAlignment", { "Enum", "TextYAlignment", "Top" } },
			{ "TextWrapped", true },
			{ "FontFace", { "Font", "rbxasset://fonts/families/SourceSansPro.json" } },
			{ "Font", { "Enum", "Font", "SourceSans" } },
		}
	do
		table.insert(variants, { node = name, prop = v[1], value = v[2] })
	end
	if class ~= "InputActionLabel" then
		table.insert(variants, { node = name, prop = "TextScaled", value = true })
	end
end

return {
	name = "Every class that draws text: the text properties",
	shows = "Four dark boxes, one per text class, each holding the same line, centred, unwrapped and overflowing the box.",
	surface = { width = 220, height = 210 },
	variants = variants,
	tree = {
		class = "Frame",
		name = "Panel",
		props = {
			Size = { "UDim2", 1, 0, 1, 0 },
			BackgroundColor3 = { "Color3", 0.10, 0.11, 0.13 },
			BorderSizePixel = 0,
		},
		children = rows,
	},
}

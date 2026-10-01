--[[
	The image properties, varied on each class that draws an image.

	`two_tone.png` is 8 by 4, red on its left half and blue on its right, so a
	source rectangle that moves is a colour that changes. Each class shows the
	left 4 by 4, the red square, stretched over an 80 by 40 box.
	`swatch.png` is plain green and stands in for a second asset.
]]
local CLASSES = { "ImageLabel", "ImageButton" }

local boxes = {}
local variants = {}

for i, class in ipairs(CLASSES) do
	local name = class .. "Picture"
	table.insert(boxes, {
		class = class,
		name = name,
		props = {
			Position = { "UDim2", 0, 10, 0, 10 + (i - 1) * 50 },
			Size = { "UDim2", 0, 80, 0, 40 },
			BackgroundColor3 = { "Color3", 0.16, 0.17, 0.20 },
			BorderSizePixel = 0,
			Image = "mod://assets/two_tone.png",
			ImageRectOffset = { "Vector2", 0, 0 },
			ImageRectSize = { "Vector2", 4, 4 },
			ImageColor3 = { "Color3", 1, 1, 1 },
			ImageTransparency = 0,
			ScaleType = { "Enum", "ScaleType", "Stretch" },
		},
	})
	for _, v in
		{
			{ "Image", "" },
			{ "ImageContent", "mod://assets/swatch.png" },
			{ "ImageRectOffset", { "Vector2", 4, 0 } },
			{ "ImageRectSize", { "Vector2", 8, 4 } },
			{ "ImageColor3", { "Color3", 0.2, 0.2, 0.2 } },
			{ "ImageTransparency", 0.6 },
			{ "ScaleType", { "Enum", "ScaleType", "Fit" } },
		}
	do
		table.insert(variants, { node = name, prop = v[1], value = v[2] })
	end
end

return {
	name = "Every class that draws an image: the image properties",
	shows = "Two 80 by 40 boxes, one per image class, each filled by the red half of a two-colour image stretched to fit.",
	surface = { width = 100, height = 110 },
	variants = variants,
	tree = {
		class = "Frame",
		name = "Panel",
		props = {
			Size = { "UDim2", 1, 0, 1, 0 },
			BackgroundColor3 = { "Color3", 0.10, 0.11, 0.13 },
			BorderSizePixel = 0,
		},
		children = boxes,
	},
}

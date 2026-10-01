--!strict
--[[
	Flex list demo: a tray of tag chips laid out by one UIListLayout, and a
	toolbar of buttons that change that layout while it runs.

	    cargo run -- examples/widgets/flex-list-demo

	Plain DataModel: `Instance.new`, properties and `Activated`, nothing imported,
	so the same file builds the same tree in a Roblox place. Nothing here
	positions a chip or a button by hand; two UIListLayouts do all of it, and the
	buttons only assign the layout's own properties.

	WHAT TO LOOK FOR, one click at a time from the start:

	  1. at rest       one line of chips in LayoutOrder, packed to the left and
	                   cut off at the tray's right edge. The taller "flex" chip
	                   sets the line's height and the others sit at its top.
	  2. Wraps         the chips that did not fit move to a second line starting
	                   at the left edge, with the list's Padding between lines.
	  3. Horizontal    (Fill) every chip on a line widens by the same amount until
	     flex          the line exactly fills the tray. Click again for
	                   SpaceAround, SpaceBetween and SpaceEvenly: the chips go
	                   back to their own widths and the spare room moves between
	                   them instead.
	  4. Line align    Center puts each short chip in the middle of its line,
	                   level with the tall chip's centre; Stretch makes every chip
	                   on that line as tall as the tallest.
	  5. Align Y       Bottom drops the whole block of lines to the bottom of the
	                   tray, keeping the first line on top. Vertical flex shares
	                   that same spare height between the lines instead.
	  6. Align X       with flex at None, Center centres each line on its own
	                   width rather than the block as a whole.

	The line under the tray always says what the layout is set to.
]]

local widget = assert(desktop.Widget, "dew.toml must grant `widget`")

local WIDTH, HEIGHT = 440, 320

local root = widget({ width = WIDTH, height = HEIGHT })

local TEXT = Color3.fromRGB(226, 232, 244)
local MUTED = Color3.fromRGB(140, 152, 176)

--- The tags, in the order they should appear. The one marked `featured` is
--- drawn larger, so a line holding it is taller than its other chips and the
--- line alignment has something to show.
local TAGS: { { text: string, featured: boolean? } } = {
	{ text = "layout" },
	{ text = "flex", featured = true },
	{ text = "Luau" },
	{ text = "accessibility" },
	{ text = "wrapping" },
	{ text = "UI" },
	{ text = "design systems" },
	{ text = "Roblox" },
	{ text = "desktop" },
}

--- The values each control steps through, in the order a click visits them.
local FLEX = {
	Enum.UIFlexAlignment.None,
	Enum.UIFlexAlignment.Fill,
	Enum.UIFlexAlignment.SpaceAround,
	Enum.UIFlexAlignment.SpaceBetween,
	Enum.UIFlexAlignment.SpaceEvenly,
}
local ALIGN_X = { Enum.HorizontalAlignment.Left, Enum.HorizontalAlignment.Center, Enum.HorizontalAlignment.Right }
local ALIGN_Y = { Enum.VerticalAlignment.Top, Enum.VerticalAlignment.Center, Enum.VerticalAlignment.Bottom }
local LINE = {
	Enum.ItemLineAlignment.Automatic,
	Enum.ItemLineAlignment.Start,
	Enum.ItemLineAlignment.Center,
	Enum.ItemLineAlignment.End,
	Enum.ItemLineAlignment.Stretch,
}

--- The item after `current` in `items`, wrapping round to the first.
local function after(items: { any }, current: any): any
	for i, item in items do
		if item == current then
			return items[i % #items + 1]
		end
	end
	return items[1]
end

local function rounded(parent: Instance, radius: number)
	local corner = Instance.new("UICorner")
	corner.CornerRadius = UDim.new(0, radius)
	corner.Parent = parent
end

local function inset(parent: Instance, x: number, y: number)
	local padding = Instance.new("UIPadding")
	padding.PaddingLeft = UDim.new(0, x)
	padding.PaddingRight = UDim.new(0, x)
	padding.PaddingTop = UDim.new(0, y)
	padding.PaddingBottom = UDim.new(0, y)
	padding.Parent = parent
end

--- A list in `parent`, in LayoutOrder.
---
--- SORTORDER DEFAULTS TO NAME, not LayoutOrder, on every layout class. A list
--- that leaves it unset is ordered alphabetically by child name, which is
--- rarely what the LayoutOrder values beside it meant.
local function list(parent: Instance, direction: Enum.FillDirection, gap: number): UIListLayout
	local layout = Instance.new("UIListLayout")
	layout.FillDirection = direction
	layout.SortOrder = Enum.SortOrder.LayoutOrder
	layout.Padding = UDim.new(0, gap)
	layout.Parent = parent
	return layout
end

local function textLabel(parent: Instance, name: string, order: number, height: number, size: number, colour: Color3): TextLabel
	local text = Instance.new("TextLabel")
	text.Name = name
	text.LayoutOrder = order
	text.Size = UDim2.new(1, 0, 0, height)
	text.BackgroundTransparency = 1
	text.TextSize = size
	text.TextColor3 = colour
	text.TextXAlignment = Enum.TextXAlignment.Left
	text.Parent = parent
	return text
end

--- A chip as wide as its own text. `AutomaticSize.X` measures the text and the
--- UIPadding together, so no width is written down anywhere.
local function chip(parent: Instance, order: number, tag: { text: string, featured: boolean? })
	local label = Instance.new("TextLabel")
	label.Name = tag.text
	label.LayoutOrder = order
	label.Size = UDim2.new(0, 0, 0, if tag.featured then 36 else 24)
	label.AutomaticSize = Enum.AutomaticSize.X
	label.BackgroundColor3 = if tag.featured then Color3.fromRGB(70, 120, 210) else Color3.fromRGB(52, 62, 84)
	label.TextColor3 = TEXT
	label.TextSize = if tag.featured then 16 else 13
	label.Text = tag.text
	label.Parent = parent
	rounded(label, if tag.featured then 10 else 8)
	inset(label, 10, 0)
end

local card = Instance.new("Frame")
card.Name = "Card"
card.Size = UDim2.fromOffset(WIDTH, HEIGHT)
card.BackgroundColor3 = Color3.fromRGB(22, 26, 36)
card.Parent = root
rounded(card, 14)
inset(card, 16, 14)
list(card, Enum.FillDirection.Vertical, 10)

textLabel(card, "Title", 1, 20, 16, TEXT).Text = "Tags"

--- THE TRAY CLIPS, so a line that does not fit is cut at the edge rather than
--- drawn over the card, which is what makes turning Wraps on visible.
local tray = Instance.new("Frame")
tray.Name = "Tray"
tray.LayoutOrder = 2
tray.Size = UDim2.new(1, 0, 0, 128)
tray.BackgroundColor3 = Color3.fromRGB(30, 36, 50)
tray.ClipsDescendants = true
tray.Parent = card
rounded(tray, 10)
inset(tray, 8, 8)

local flow = list(tray, Enum.FillDirection.Horizontal, 6)
for order, tag in TAGS do
	chip(tray, order, tag)
end

local settings = textLabel(card, "Settings", 3, 16, 12, MUTED)

--- THE TOOLBAR IS A WRAPPING LIST TOO: six buttons do not fit on one line of
--- this card, and nothing here has to know where the second line starts.
local toolbar = Instance.new("Frame")
toolbar.Name = "Toolbar"
toolbar.LayoutOrder = 4
toolbar.Size = UDim2.new(1, 0, 0, 64)
toolbar.BackgroundTransparency = 1
toolbar.Parent = card
list(toolbar, Enum.FillDirection.Horizontal, 6).Wraps = true

local function control(name: string, order: number, text: string, onActivated: () -> ())
	local button = Instance.new("TextButton")
	button.Name = name
	button.LayoutOrder = order
	button.Size = UDim2.new(0, 0, 0, 28)
	button.AutomaticSize = Enum.AutomaticSize.X
	button.BackgroundColor3 = Color3.fromRGB(44, 58, 84)
	button.TextColor3 = TEXT
	button.TextSize = 12
	button.Text = text
	button.Parent = toolbar
	rounded(button, 8)
	inset(button, 12, 0)
	button.Activated:Connect(onActivated)
end

control("Wraps", 1, "Wraps", function()
	flow.Wraps = not flow.Wraps
end)
control("HorizontalFlex", 2, "Horizontal flex", function()
	flow.HorizontalFlex = after(FLEX, flow.HorizontalFlex)
end)
control("VerticalFlex", 3, "Vertical flex", function()
	flow.VerticalFlex = after(FLEX, flow.VerticalFlex)
end)
control("AlignX", 4, "Align X", function()
	flow.HorizontalAlignment = after(ALIGN_X, flow.HorizontalAlignment)
end)
control("AlignY", 5, "Align Y", function()
	flow.VerticalAlignment = after(ALIGN_Y, flow.VerticalAlignment)
end)
control("LineAlign", 6, "Line align", function()
	flow.ItemLineAlignment = after(LINE, flow.ItemLineAlignment)
end)

--- ONE PLACE WRITES THE SETTINGS LINE, and it listens to the layout rather than
--- to the buttons, so it stays true whatever changed the layout.
local function describe()
	settings.Text = string.format(
		"Wraps %s  |  flex %s, %s  |  align %s, %s  |  line %s",
		if flow.Wraps then "on" else "off",
		flow.HorizontalFlex.Name,
		flow.VerticalFlex.Name,
		flow.HorizontalAlignment.Name,
		flow.VerticalAlignment.Name,
		flow.ItemLineAlignment.Name
	)
end
flow.Changed:Connect(describe)
describe()

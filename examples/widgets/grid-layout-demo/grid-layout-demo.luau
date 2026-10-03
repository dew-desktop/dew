--!strict
--[[
	Grid layout demo: an app launcher whose tiles are placed by one
	UIGridLayout, and a toolbar of buttons that change that layout while it runs.

	    cargo run -- examples/widgets/grid-layout-demo
	    rojo serve examples/widgets/grid-layout-demo

	Plain DataModel: `Instance.new`, properties and `Activated`, nothing imported,
	so the same file builds the same tree in a Roblox place. No tile has a
	Position or a Size of its own; the grid gives every tile its cell, and the
	buttons only assign the grid's own properties.

	WHAT TO LOOK FOR, one click at a time from the start:

	  1. at rest       eight tiles in 72 pixel cells with 8 pixels between them,
	                   five to a row because five is what fits, filled left to
	                   right from the top left. Each icon stays square inside
	                   its tile because of a UIAspectRatioConstraint.
	  2. a tile        clicking a tile selects it: it lights up and the line
	                   under the launcher names it.
	  3. Size          the cells become 56 square, then 112 by 56, then 72 again.
	                   The grid fits as many to a row as the width allows, so a
	                   smaller tile means more columns and a wide one fewer.
	  4. Columns       FillDirectionMaxCells caps each row at 4, then 3, before
	                   the width would; "auto" lifts the cap.
	  5. Corner        the first tile moves to another corner of the block of
	                   tiles, not of the launcher, and the order runs back from
	                   there: from TopRight, Mail is at the right of the first row
	                   and Calendar to its left.
	  6. Fill          Vertical fills each column top to bottom before moving
	                   across, three to a column because three fit the height.

	The line under the launcher always says what the grid is set to.
]]

local WIDTH, HEIGHT = 480, 380

--- WHERE THE TREE GOES is the one thing that depends on the host. Dew installs
--- `desktop`, and a widget of this size is the root. A Roblox place has no
--- `desktop`, so the root is a Frame of the same size, centred in a ScreenGui
--- in this player's PlayerGui, below the engine's top bar.
local root: Instance
if desktop then
	local widget = assert(desktop.Widget, "dew.toml must grant `widget`")
	root = widget({ width = WIDTH, height = HEIGHT })
else
	local screen = Instance.new("ScreenGui")
	screen.Name = "GridLayoutDemo"
	screen.ResetOnSpawn = false
	local frame = Instance.new("Frame")
	frame.Name = "Root"
	frame.AnchorPoint = Vector2.new(0.5, 0.5)
	frame.Position = UDim2.fromScale(0.5, 0.5)
	frame.Size = UDim2.fromOffset(WIDTH, HEIGHT)
	frame.BackgroundTransparency = 1
	frame.Parent = screen
	screen.Parent = game:GetService("Players").LocalPlayer:WaitForChild("PlayerGui")
	root = frame
end

local TEXT = Color3.fromRGB(226, 232, 244)
local MUTED = Color3.fromRGB(140, 152, 176)
local TILE = Color3.fromRGB(36, 42, 58)
local SELECTED = Color3.fromRGB(58, 74, 108)

--- The apps, in launcher order: a name, the two letters on its icon, and the icon's
--- colour.
local APPS: { { name: string, glyph: string, colour: Color3 } } = {
	{ name = "Mail", glyph = "Ma", colour = Color3.fromRGB(70, 120, 210) },
	{ name = "Calendar", glyph = "Ca", colour = Color3.fromRGB(214, 84, 84) },
	{ name = "Music", glyph = "Mu", colour = Color3.fromRGB(196, 92, 170) },
	{ name = "Photos", glyph = "Ph", colour = Color3.fromRGB(232, 168, 60) },
	{ name = "Notes", glyph = "No", colour = Color3.fromRGB(222, 196, 80) },
	{ name = "Maps", glyph = "Mp", colour = Color3.fromRGB(80, 170, 110) },
	{ name = "Weather", glyph = "We", colour = Color3.fromRGB(70, 170, 200) },
	{ name = "Settings", glyph = "Se", colour = Color3.fromRGB(120, 128, 150) },
}

--- The values each control steps through, in the order a click visits them.
local SIZES = { UDim2.fromOffset(72, 72), UDim2.fromOffset(56, 56), UDim2.fromOffset(112, 56) }
local COLUMNS = { 0, 4, 3 }
local CORNERS = {
	Enum.StartCorner.TopLeft,
	Enum.StartCorner.TopRight,
	Enum.StartCorner.BottomRight,
	Enum.StartCorner.BottomLeft,
}
local FILLS = { Enum.FillDirection.Horizontal, Enum.FillDirection.Vertical }

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
--- SORTORDER DEFAULTS TO NAME, not LayoutOrder, on every layout class, the
--- grid included. A layout that leaves it unset is ordered alphabetically by
--- child name, which is rarely what the LayoutOrder values beside it meant.
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

local card = Instance.new("Frame")
card.Name = "Card"
card.Size = UDim2.fromOffset(WIDTH, HEIGHT)
card.BackgroundColor3 = Color3.fromRGB(22, 26, 36)
card.Parent = root
rounded(card, 14)
inset(card, 16, 14)
list(card, Enum.FillDirection.Vertical, 10)

textLabel(card, "Title", 1, 20, 16, TEXT).Text = "Apps"

--- THE LAUNCHER CLIPS, so a tile that does not fit is cut at the edge rather
--- than drawn over the card.
local launcher = Instance.new("Frame")
launcher.Name = "Launcher"
launcher.LayoutOrder = 2
launcher.Size = UDim2.new(1, 0, 0, 248)
launcher.BackgroundColor3 = Color3.fromRGB(30, 36, 50)
launcher.ClipsDescendants = true
launcher.Parent = card
rounded(launcher, 10)
inset(launcher, 8, 8)

--- THE ONLY THING THAT PLACES A TILE. CellSize and CellPadding are the whole
--- of each tile's geometry; FillDirection is set here because a grid's default
--- (Horizontal) is not a list's (Vertical), and saying it keeps the reader from
--- having to know that.
local grid = Instance.new("UIGridLayout")
grid.Name = "Grid"
grid.SortOrder = Enum.SortOrder.LayoutOrder
grid.CellSize = SIZES[1]
grid.CellPadding = UDim2.fromOffset(8, 8)
grid.FillDirection = Enum.FillDirection.Horizontal
grid.Parent = launcher

local status = textLabel(card, "Status", 3, 16, 12, MUTED)

local selected: TextButton? = nil
local tiles: { TextButton } = {}

--- One tile: a button with a square icon above its name. The icon is sized by
--- scale from the tile and kept square by a UIAspectRatioConstraint, so it
--- stays square in a wide cell as well as a square one.
local function tile(order: number, app: { name: string, glyph: string, colour: Color3 }): TextButton
	local button = Instance.new("TextButton")
	button.Name = app.name
	button.LayoutOrder = order
	button.Text = ""
	button.BackgroundColor3 = TILE
	button.Parent = launcher
	rounded(button, 10)

	local outline = Instance.new("UIStroke")
	outline.Name = "Outline"
	outline.ApplyStrokeMode = Enum.ApplyStrokeMode.Border
	outline.Color = Color3.fromRGB(120, 170, 255)
	outline.Thickness = 2
	outline.Transparency = 1
	outline.Parent = button

	local icon = Instance.new("TextLabel")
	icon.Name = "Icon"
	icon.AnchorPoint = Vector2.new(0.5, 0)
	icon.Position = UDim2.new(0.5, 0, 0, 6)
	icon.Size = UDim2.new(1, -16, 1, -30)
	icon.BackgroundColor3 = app.colour
	icon.TextColor3 = TEXT
	icon.TextSize = 14
	icon.Text = app.glyph
	icon.Parent = button
	rounded(icon, 8)
	local square = Instance.new("UIAspectRatioConstraint")
	square.AspectRatio = 1
	square.Parent = icon

	local label = Instance.new("TextLabel")
	label.Name = "Label"
	label.Position = UDim2.new(0, 0, 1, -20)
	label.Size = UDim2.new(1, 0, 0, 16)
	label.BackgroundTransparency = 1
	label.TextColor3 = TEXT
	label.TextSize = 11
	label.Text = app.name
	label.Parent = button

	return button
end

--- ONE PLACE WRITES THE STATUS LINE, and it listens to the grid rather than
--- to the buttons, so it stays true whatever changed the grid.
local function describe()
	status.Text = string.format(
		"tiles %d x %d  |  columns %s  |  from %s  |  fill %s  |  selected %s",
		grid.CellSize.X.Offset,
		grid.CellSize.Y.Offset,
		if grid.FillDirectionMaxCells == 0 then "auto" else tostring(grid.FillDirectionMaxCells),
		grid.StartCorner.Name,
		grid.FillDirection.Name,
		if selected then selected.Name else "none"
	)
end

local function choose(button: TextButton)
	selected = button
	for _, other in tiles do
		local chosen = other == button
		other.BackgroundColor3 = if chosen then SELECTED else TILE
		local outline = other:FindFirstChild("Outline") :: UIStroke
		outline.Transparency = if chosen then 0 else 1
	end
	describe()
end

for order, app in APPS do
	local button = tile(order, app)
	table.insert(tiles, button)
	button.Activated:Connect(function()
		choose(button)
	end)
end

--- The toolbar is a row of its own, laid out by a list.
local toolbar = Instance.new("Frame")
toolbar.Name = "Toolbar"
toolbar.LayoutOrder = 4
toolbar.Size = UDim2.new(1, 0, 0, 28)
toolbar.BackgroundTransparency = 1
toolbar.Parent = card
list(toolbar, Enum.FillDirection.Horizontal, 6)

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

control("SizeButton", 1, "Size", function()
	grid.CellSize = after(SIZES, grid.CellSize)
end)
control("ColumnsButton", 2, "Columns", function()
	grid.FillDirectionMaxCells = after(COLUMNS, grid.FillDirectionMaxCells)
end)
control("CornerButton", 3, "Corner", function()
	grid.StartCorner = after(CORNERS, grid.StartCorner)
end)
control("FillButton", 4, "Fill", function()
	grid.FillDirection = after(FILLS, grid.FillDirection)
end)

grid.Changed:Connect(describe)
describe()

--- A Roblox ModuleScript must return exactly one value, so this file returns
--- the card it built.
return card

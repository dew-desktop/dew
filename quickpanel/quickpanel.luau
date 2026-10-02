--!strict
--[[
	The Dew quick panel: a small floating widget that shows every installed
	applet as a card, says whether each one is running, and turns it on or
	off with a switch. Everything else (installing, uninstalling, signing in)
	belongs to the dashboard.

	The widget drags from anywhere on its body, snaps to a screen edge, stays
	on screen and reopens where it was left. Those are host behaviours asked
	for in the `widget` call below; nothing here moves the window.

	`dew.Library` is a host-only permission, so the host loads this applet
	only from its own bundled directory, the same as the dashboard. From a
	checkout, `cargo run -- quickpanel` is refused for that reason, and
	`cargo run -- test quickpanel` runs it against a fake library instead.

	One state source, `entries`, holds the last `Library.List()`. The click
	handler and `Library.OnChange` only write it; one effect rebuilds the
	cards from it.
]]

local vide = require("./roblox_packages/vide")

local widget = assert(desktop.Widget, "dew.toml must grant `widget`")
local library = assert(dew.Library, "dew.toml must grant `library`")

type Entry = { id: string, name: string, description: string, enabled: boolean, running: boolean }

-- THE TOKENS, copied by name and value from the token sheet in
-- `dashboard/dashboard.luau`, so the two surfaces read as one product.
local T = {
	BackgroundColor = Color3.fromRGB(18, 20, 26),
	SurfaceColor = Color3.fromRGB(28, 31, 40),
	TextPrimary = Color3.fromRGB(233, 239, 249),
	TextSecondary = Color3.fromRGB(146, 162, 188),
	TextPlaceholder = Color3.fromRGB(110, 122, 140),
	AccentColor = Color3.fromRGB(38, 52, 74),
	SuccessColor = Color3.fromRGB(48, 90, 64),
	SuccessText = Color3.fromRGB(214, 240, 224),
	ErrorText = Color3.fromRGB(224, 148, 148),
	Line = Color3.fromRGB(38, 42, 54),
}

local function face(family: string, weight: Enum.FontWeight?): Font
	return Font.new(`rbxasset://fonts/families/{family}.json`, weight or Enum.FontWeight.Regular)
end
local HEADING = face("Montserrat", Enum.FontWeight.Bold)
local BODY = face("Arimo")

local WIDTH, HEIGHT = 320, 420
-- The width a card starts from: room for its icon and switch side by side.
-- Each row's spare width is shared out by the grid's Fill, so two cards fit
-- a row from 258 wide up to 375, and three from there.
local CARD_WIDTH = 112
local CARD_HEIGHT = 96

local root = widget({
	width = WIDTH,
	height = HEIGHT,
	anchor = "top-right",
	offset = { x = 24, y = 24 },
	zOrder = "topmost",
	draggable = true,
	snapToEdges = true,
	keepOnScreen = true,
	savePosition = true,
})

--------------------------------------------------------------------------
-- BUILDING BLOCKS. No `Position` anywhere: every placement comes from a
-- list layout, a flex item or a fixed size.
--------------------------------------------------------------------------

local function frame(parent: Instance, name: string, order: number, size: UDim2, colour: Color3?): Frame
	local f = Instance.new("Frame")
	f.Name = name
	f.LayoutOrder = order
	f.Size = size
	f.BorderSizePixel = 0
	if colour then
		f.BackgroundColor3 = colour
	else
		f.BackgroundTransparency = 1
	end
	f.Parent = parent
	return f
end

local function list(parent: Instance, direction: Enum.FillDirection, gap: number): UIListLayout
	local layout = Instance.new("UIListLayout")
	layout.FillDirection = direction
	layout.SortOrder = Enum.SortOrder.LayoutOrder
	layout.Padding = UDim.new(0, gap)
	layout.Parent = parent
	return layout
end

local function pad(parent: Instance, x: number, y: number)
	local p = Instance.new("UIPadding")
	p.PaddingLeft = UDim.new(0, x)
	p.PaddingRight = UDim.new(0, x)
	p.PaddingTop = UDim.new(0, y)
	p.PaddingBottom = UDim.new(0, y)
	p.Parent = parent
end

local function corner(parent: Instance, radius: number)
	local c = Instance.new("UICorner")
	c.CornerRadius = UDim.new(0, radius)
	c.Parent = parent
end

local function grow(child: Instance)
	local item = Instance.new("UIFlexItem")
	item.FlexMode = Enum.UIFlexMode.Grow
	item.Parent = child
end

-- One line of text that never overflows: it is cut with an ellipsis at the
-- box's edge. The box is one line tall (a modern face's line is its size).
local function text(parent: Instance, name: string, order: number, size: number, font: Font, colour: Color3): TextLabel
	local t = Instance.new("TextLabel")
	t.Name = name
	t.LayoutOrder = order
	t.Size = UDim2.new(1, 0, 0, size)
	t.BackgroundTransparency = 1
	t.TextSize = size
	t.FontFace = font
	t.TextColor3 = colour
	t.TextXAlignment = Enum.TextXAlignment.Left
	t.TextYAlignment = Enum.TextYAlignment.Center
	t.TextTruncate = Enum.TextTruncate.AtEnd
	t.Parent = parent
	return t
end

-- Library entries carry no icon, so a card shows a tile with the applet's
-- initials, coloured by a hue taken from its id so it stays the same
-- between launches.
local function initials(name: string): string
	local out = ""
	for word in string.gmatch(name, "%w+") do
		out ..= string.upper(string.sub(word, 1, 1))
		if #out == 2 then
			break
		end
	end
	return if out == "" then "?" else out
end

local function hue(id: string): number
	local sum = 0
	for i = 1, #id do
		sum = (sum * 31 + string.byte(id, i)) % 360
	end
	return sum / 360
end

local function monogram(parent: Instance, order: number, entry: Entry)
	local tile = frame(parent, "Icon", order, UDim2.fromOffset(30, 30), Color3.fromHSV(hue(entry.id), 0.45, 0.62))
	corner(tile, 8)
	local glyph = Instance.new("TextLabel")
	glyph.Name = "Glyph"
	glyph.Size = UDim2.fromScale(1, 1)
	glyph.BackgroundTransparency = 1
	glyph.Text = initials(entry.name)
	glyph.TextSize = 12
	glyph.FontFace = HEADING
	glyph.TextColor3 = Color3.fromRGB(250, 250, 252)
	glyph.Parent = tile
end

--------------------------------------------------------------------------
-- THE PANEL
--------------------------------------------------------------------------

-- SIZED IN PIXELS, NOT AS A SCALE OF THE ROOT. The widget is a fixed size
-- anyway, and `dew test`'s pointer hit-tests the tree at a zero-sized root,
-- where a scale-sized body would have nothing in it to click.
local body = frame(root, "Body", 0, UDim2.fromOffset(WIDTH, HEIGHT), T.BackgroundColor)
corner(body, 14)
local outline = Instance.new("UIStroke")
outline.Color = T.Line
outline.Thickness = 1
outline.Parent = body
list(body, Enum.FillDirection.Vertical, 0)
-- The scrolling shelf stops short of the bottom edge, so a card scrolled
-- under it never covers the body's rounded corners.
local inset = Instance.new("UIPadding")
inset.PaddingBottom = UDim.new(0, 10)
inset.Parent = body

local header = frame(body, "Header", 1, UDim2.new(1, 0, 0, 54))
pad(header, 14, 0)
local headerLayout = list(header, Enum.FillDirection.Vertical, 5)
headerLayout.VerticalAlignment = Enum.VerticalAlignment.Center
local title = text(header, "Title", 1, 16, HEADING, T.TextPrimary)
title.Text = "Applets"
local count = text(header, "Count", 2, 12, BODY, T.TextSecondary)

frame(body, "Rule", 2, UDim2.new(1, 0, 0, 1), T.Line)

local shelf = Instance.new("ScrollingFrame")
shelf.Name = "Shelf"
shelf.LayoutOrder = 3
shelf.Size = UDim2.fromScale(1, 0)
shelf.BackgroundTransparency = 1
shelf.BorderSizePixel = 0
shelf.ScrollBarThickness = 4
shelf.CanvasSize = UDim2.fromScale(1, 0)
shelf.AutomaticCanvasSize = Enum.AutomaticSize.Y
shelf.Parent = body
grow(shelf)
pad(shelf, 12, 12)

local grid = frame(shelf, "Grid", 1, UDim2.fromScale(1, 0))
grid.AutomaticSize = Enum.AutomaticSize.Y
local gridLayout = list(grid, Enum.FillDirection.Horizontal, 10)
gridLayout.Wraps = true
gridLayout.HorizontalFlex = Enum.UIFlexAlignment.Fill

-- Wrapped and grown to fit rather than cut, since it is the only thing on
-- the shelf when it shows.
local empty = text(shelf, "Empty", 2, 13, BODY, T.TextSecondary)
empty.Text = "No applets installed yet. The dashboard's Store has some to try."
empty.TextTruncate = Enum.TextTruncate.None
empty.TextWrapped = true
empty.TextYAlignment = Enum.TextYAlignment.Top
empty.Size = UDim2.fromScale(1, 0)
empty.AutomaticSize = Enum.AutomaticSize.Y

-- A failed switch says why here, on one line, until the next action.
local notice = text(body, "Notice", 4, 12, BODY, T.ErrorText)
notice.Size = UDim2.new(1, 0, 0, 30)
pad(notice, 14, 0)
notice.Visible = false

local function say(message: string?)
	notice.Text = message or ""
	notice.Visible = message ~= nil
end

--------------------------------------------------------------------------
-- STATE
--------------------------------------------------------------------------

local entries = vide.source(library.List() :: { Entry })

local function refresh()
	entries(library.List() :: { Entry })
end

-- The switch follows `running`, not `enabled`: off always starts the
-- applet and on always stops it, so an applet that is enabled but was
-- closed by hand reads as off and one click brings it back.
local function toggle(entry: Entry)
	local result = library.SetEnabled(entry.id, not entry.running)
	say(if result.ok then nil else result.error)
	refresh()
end

-- A switch whose knob is placed by its own list's alignment: left when
-- off, right when on.
local function switch(parent: Instance, order: number, entry: Entry): TextButton
	local track = Instance.new("TextButton")
	track.Name = "Toggle"
	track.LayoutOrder = order
	track.Size = UDim2.fromOffset(36, 20)
	track.AutoButtonColor = false
	track.Text = ""
	track.BorderSizePixel = 0
	track.BackgroundColor3 = if entry.running then T.SuccessColor else T.AccentColor
	track.Parent = parent
	corner(track, 10)
	pad(track, 3, 3)
	local layout = list(track, Enum.FillDirection.Horizontal, 0)
	layout.VerticalAlignment = Enum.VerticalAlignment.Center
	layout.HorizontalAlignment = if entry.running then Enum.HorizontalAlignment.Right else Enum.HorizontalAlignment.Left
	local knob = frame(track, "Knob", 1, UDim2.fromOffset(14, 14), if entry.running then T.SuccessText else T.TextSecondary)
	corner(knob, 7)
	track.Activated:Connect(function()
		toggle(entry)
	end)
	return track
end

local function card(order: number, entry: Entry)
	local c = frame(grid, "Card", order, UDim2.fromOffset(CARD_WIDTH, CARD_HEIGHT), T.SurfaceColor)
	corner(c, 10)
	pad(c, 12, 12)
	list(c, Enum.FillDirection.Vertical, 8)

	local top = frame(c, "Top", 1, UDim2.new(1, 0, 0, 30))
	local topLayout = list(top, Enum.FillDirection.Horizontal, 8)
	topLayout.VerticalAlignment = Enum.VerticalAlignment.Center
	monogram(top, 1, entry)
	grow(frame(top, "Spacer", 2, UDim2.fromOffset(0, 1)))
	switch(top, 3, entry)

	local name = text(c, "Name", 2, 14, HEADING, T.TextPrimary)
	name.Text = entry.name

	local state = frame(c, "State", 3, UDim2.new(1, 0, 0, 12))
	local stateLayout = list(state, Enum.FillDirection.Horizontal, 6)
	stateLayout.VerticalAlignment = Enum.VerticalAlignment.Center
	local dot = frame(state, "Dot", 1, UDim2.fromOffset(6, 6), if entry.running then T.SuccessText else T.TextPlaceholder)
	corner(dot, 3)
	local label = text(state, "Label", 2, 12, BODY, if entry.running then T.SuccessText else T.TextSecondary)
	label.Size = UDim2.new(1, -12, 0, 12)
	label.Text = if entry.running then "Running" else "Stopped"
end

vide.root(function()
	vide.effect(function()
		local current = entries()
		for _, child in grid:GetChildren() do
			if child:IsA("Frame") then
				child:Destroy()
			end
		end

		local running = 0
		for i, entry in current do
			card(i, entry)
			if entry.running then
				running += 1
			end
		end

		count.Text = `{#current} installed, {running} running`
		grid.Visible = #current > 0
		empty.Visible = #current == 0
	end)
end)

-- Every change to the library, whoever made it (this panel, the dashboard,
-- `dew install` in a terminal), arrives here.
library.OnChange(refresh)

--!strict
--[[
	The Dew dashboard: Library, Store and Account in one desktop window.

	A sidebar holds the navigation, a count per section and the signed in
	account at its foot. The active section takes the rest of the width. Every
	placement comes from a layout container (UIListLayout, UIFlexItem,
	UISizeConstraint, UIPadding, AutomaticSize), never from a Position, so the
	window can be resized freely: rows put their buttons under the text and
	form lines put the field under its label when the window is narrow.

	Text is handled on purpose everywhere. A name or an id is one line and
	truncates; a description, a status or an error wraps and grows.

	Bundled with the host rather than installed like a marketplace package:
	`dew.toml` asks for `discover`, `install`, `library` and `auth`, host
	permissions no ordinary applet can be granted.
]]

local vide = require("./roblox_packages/vide")
local source = vide.source
local effect = vide.effect
local create = vide.create

local Aether = require("./roblox_packages/aether")
local Tabs = Aether.Tabs

local window = assert(desktop.Window, "dew.toml must grant `window`")
local marketplace = assert(dew.Marketplace, "dew.toml must grant `discover` and `install`")
local library = assert(dew.Library, "dew.toml must grant `library`")
local account = assert(dew.Account, "dew.toml must grant `auth`")

-- 720 WIDE: the dashboard is the platform's full window. At this width a row
-- is one line (icon, text, buttons) and the sidebar sits at its 200 cap. The
-- window stays resizable, and the layout below holds from about 420 up.
local root = window({ title = "Dew Dashboard", width = 720, height = 560 })

-- `dom.luau` parents the recipe StyleSheet under `root` as soon as it exists,
-- which the cascade needs before any `Bind` below resolves a token.
local newDom = require("./dom")
local dom = newDom(root)

--------------------------------------------------------------------------
-- DESIGN TOKENS. One StyleSheet of named attributes; every colour and
-- radius below reaches an instance through a recipe that names a token.
--------------------------------------------------------------------------

local design = dom.New("StyleSheet")
design.Name = "Design"
-- The window behind everything.
design:SetAttribute("BackgroundColor", Color3.fromRGB(18, 20, 26))
-- Rows, sections, text fields and inactive chips: one raised surface tone.
design:SetAttribute("SurfaceColor", Color3.fromRGB(28, 31, 40))
-- Rows and sections.
design:SetAttribute("RowRadius", UDim.new(0, 8))
-- Buttons, fields and navigation items.
design:SetAttribute("ControlRadius", UDim.new(0, 6))
-- Titles, names and field text.
design:SetAttribute("TextPrimary", Color3.fromRGB(233, 239, 249))
-- Subtitles, descriptions, ids and field labels.
design:SetAttribute("TextSecondary", Color3.fromRGB(146, 162, 188))
-- TextBox placeholder text.
design:SetAttribute("TextPlaceholder", Color3.fromRGB(110, 122, 140))
-- The default button tone, icon tiles and the active count pill.
design:SetAttribute("AccentColor", Color3.fromRGB(38, 52, 74))
-- Text on AccentColor, status lines and the brand mark.
design:SetAttribute("AccentText", Color3.fromRGB(206, 222, 246))
-- The danger button tone: Uninstall, Sign out.
design:SetAttribute("DangerColor", Color3.fromRGB(90, 48, 48))
design:SetAttribute("DangerText", Color3.fromRGB(240, 214, 214))
-- The success button tone: Install.
design:SetAttribute("SuccessColor", Color3.fromRGB(48, 90, 64))
design:SetAttribute("SuccessText", Color3.fromRGB(214, 240, 224))
-- Error messages, distinct from DangerText on a button.
design:SetAttribute("ErrorText", Color3.fromRGB(224, 148, 148))
-- The active navigation item and filter chip, and field outlines.
design:SetAttribute("TabActiveColor", Color3.fromRGB(48, 66, 94))
-- The sidebar, and an inactive navigation item on it.
design:SetAttribute("TabInactiveColor", Color3.fromRGB(26, 29, 38))

type Package = { ownerUserId: string, appletId: string, uploadedAt: string }
type LibraryEntry = {
	id: string,
	name: string,
	description: string,
	enabled: boolean,
	running: boolean,
	-- Ships with Dew. The host refuses to uninstall it, so its row has no
	-- Uninstall button.
	bundled: boolean?,
}
type Session = { email: string, userId: string? }
type Tone = "default" | "danger" | "success"

--------------------------------------------------------------------------
-- RECIPES. Each is bound to every instance that shows one of its
-- properties: a button's Frame reads BackgroundColor3, its label reads
-- TextColor3 and its UICorner reads CornerRadius, all from one rule.
--
-- KEPT TO FEW RULES ON PURPOSE. Each distinct selection is one StyleRule,
-- and Dew resolves the cascade afresh for every property an element did not
-- set, against every rule in the window, on every layout pass. So an element
-- that wants a colour another recipe already resolves (the sidebar, the
-- divider, an icon tile) binds that rule rather than adding one.
--------------------------------------------------------------------------

-- A sidebar navigation item, its label and its corner. The sidebar itself
-- binds the inactive state for its background.
local tabRecipe = Aether.Core.createRecipe({
	base = { BackgroundColor3 = "$TabInactiveColor", TextColor3 = "$TextSecondary", CornerRadius = "$ControlRadius" },
	variants = {
		state = {
			active = { BackgroundColor3 = "$TabActiveColor", TextColor3 = "$TextPrimary" },
			inactive = {},
		},
	},
}, { Dom = dom, Component = "DashboardTab" })
-- Called once, before any Bind: the shared StyleDerive it creates is what
-- every recipe below resolves its tokens against.
tabRecipe.SetThemeSource(design)

-- A Library filter chip, and the count pill beside a navigation item.
local chipRecipe = Aether.Core.createRecipe({
	variants = {
		state = {
			active = { BackgroundColor3 = "$TabActiveColor", TextColor3 = "$TextPrimary" },
			inactive = { BackgroundColor3 = "$SurfaceColor", TextColor3 = "$TextSecondary" },
		},
	},
}, { Dom = dom, Component = "DashboardChip" })

-- The window behind everything.
local windowRecipe = Aether.Core.createRecipe({
	base = { BackgroundColor3 = "$BackgroundColor" },
}, { Dom = dom, Component = "DashboardWindow" })

-- A Library or Store row, an Account section, the account card and the
-- divider beside the sidebar.
local rowRecipe = Aether.Core.createRecipe({
	base = { BackgroundColor3 = "$SurfaceColor", CornerRadius = "$RowRadius" },
}, { Dom = dom, Component = "DashboardRow" })

-- A button, its label and its corner. Icon tiles and the brand mark use the
-- default tone too.
local buttonRecipe = Aether.Core.createRecipe({
	base = { CornerRadius = "$ControlRadius" },
	variants = {
		tone = {
			default = { BackgroundColor3 = "$AccentColor", TextColor3 = "$AccentText" },
			danger = { BackgroundColor3 = "$DangerColor", TextColor3 = "$DangerText" },
			success = { BackgroundColor3 = "$SuccessColor", TextColor3 = "$SuccessText" },
		},
	},
}, { Dom = dom, Component = "DashboardButton" })

local textRecipe = Aether.Core.createRecipe({
	variants = {
		tone = {
			primary = { TextColor3 = "$TextPrimary" },
			secondary = { TextColor3 = "$TextSecondary" },
			accent = { TextColor3 = "$AccentText" },
			error = { TextColor3 = "$ErrorText" },
		},
	},
}, { Dom = dom, Component = "DashboardText" })

-- A TextBox, its corner and its outline.
local textInputRecipe = Aether.Core.createRecipe({
	base = {
		BackgroundColor3 = "$SurfaceColor",
		TextColor3 = "$TextPrimary",
		PlaceholderColor3 = "$TextPlaceholder",
		CornerRadius = "$ControlRadius",
		Color = "$TabActiveColor",
	},
}, { Dom = dom, Component = "DashboardTextInput" })

--------------------------------------------------------------------------
-- BUILDING BLOCKS. None takes a Position.
--------------------------------------------------------------------------

local function face(weight: Enum.FontWeight?): Font
	return Font.new("rbxasset://fonts/families/Arimo.json", weight or Enum.FontWeight.Regular)
end
local REGULAR = face()
local BOLD = face(Enum.FontWeight.Bold)
local MONO = Font.new("rbxasset://fonts/families/RobotoMono.json")

local ICON_SIZE = 36
local ROW_GAP = 12
local BUTTON_HEIGHT = 30
local BUTTON_GAP = 6
local FIELD_HEIGHT = 34

-- Button widths, each wide enough for its longest label at 13 px bold.
local WIDE = 92 -- "Uninstall"
local NARROW = 80 -- "Disable", "Enable", "Install", "Cancel", "Refresh", "Sign out"

local function frame(parent: Instance, name: string, order: number, size: UDim2): Frame
	local f = Instance.new("Frame")
	f.Name = name
	f.LayoutOrder = order
	f.Size = size
	f.BorderSizePixel = 0
	f.BackgroundTransparency = 1
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

local function pad(parent: Instance, left: number, top: number?, right: number?, bottom: number?)
	local p = Instance.new("UIPadding")
	p.PaddingLeft = UDim.new(0, left)
	p.PaddingTop = UDim.new(0, top or left)
	p.PaddingRight = UDim.new(0, right or left)
	p.PaddingBottom = UDim.new(0, bottom or top or left)
	p.Parent = parent
end

local function grow(child: Instance)
	local item = Instance.new("UIFlexItem")
	item.FlexMode = Enum.UIFlexMode.Grow
	item.Parent = child
end

local function atLeast(child: Instance, minWidth: number)
	local c = Instance.new("UISizeConstraint")
	c.MinSize = Vector2.new(minWidth, 0)
	c.Parent = child
end

local function corner(parent: Instance, radius: UDim?): UICorner
	local c = Instance.new("UICorner")
	if radius then
		c.CornerRadius = radius
	end
	c.Parent = parent
	return c
end

-- One line of text, as tall as `height`, cut with an ellipsis when it does
-- not fit.
local function line(
	parent: Instance,
	name: string,
	order: number,
	size: number,
	font: Font,
	tone: string?,
	height: number?
): TextLabel
	local t = Instance.new("TextLabel")
	t.Name = name
	t.LayoutOrder = order
	t.BackgroundTransparency = 1
	t.TextSize = size
	t.FontFace = font
	t.TextXAlignment = Enum.TextXAlignment.Left
	t.TextYAlignment = Enum.TextYAlignment.Center
	t.TextTruncate = Enum.TextTruncate.AtEnd
	t.Size = UDim2.new(1, 0, 0, height or math.ceil(size * 1.25))
	t.Parent = parent
	-- No tone leaves the colour to a recipe the caller binds instead.
	if tone then
		textRecipe.Bind(t, { tone = tone })
	end
	return t
end

-- Wrapped text that grows to however many lines it needs.
local function paragraph(parent: Instance, name: string, order: number, size: number, tone: string): TextLabel
	local t = Instance.new("TextLabel")
	t.Name = name
	t.LayoutOrder = order
	t.BackgroundTransparency = 1
	t.TextSize = size
	t.FontFace = REGULAR
	t.TextXAlignment = Enum.TextXAlignment.Left
	t.TextYAlignment = Enum.TextYAlignment.Top
	t.TextWrapped = true
	t.Size = UDim2.fromScale(1, 0)
	t.AutomaticSize = Enum.AutomaticSize.Y
	t.Parent = parent
	textRecipe.Bind(t, { tone = tone })
	return t
end

-- A status line is hidden while empty, so it leaves no gap in its list.
local function setMessage(label: TextLabel, message: string)
	label.Text = message
	label.Visible = message ~= ""
end

-- A button of a fixed width, its label centred. Needs an active vide scope:
-- `Aether.Pressable` registers its own cleanup.
--
-- FIXED, NOT AutomaticSize X. Dew measures an AutomaticSize child of a list
-- by laying out its whole subtree once more, so each nested AutomaticSize
-- level multiplies the work. A button is the deepest thing in a row, and a
-- row is repeated for every applet.
local function actionButton(
	parent: Instance,
	name: string,
	order: number,
	text: string,
	tone: Tone,
	width: number,
	onActivated: () -> ()
): (Frame, TextLabel)
	local b = Aether.Pressable({
		Parent = parent,
		Name = name,
		LayoutOrder = order,
		Size = UDim2.fromOffset(width, BUTTON_HEIGHT),
		BackgroundTransparency = 0,
		OnActivated = onActivated,
	}) :: Frame
	-- Pixels rather than scale, like the button: a scrolling list lays its
	-- rows out twice when anything inside them is sized by scale Y.
	local label = Instance.new("TextLabel")
	label.Name = "Label"
	label.Size = UDim2.fromOffset(width, BUTTON_HEIGHT)
	label.BackgroundTransparency = 1
	label.FontFace = BOLD
	label.TextSize = 13
	label.Text = text
	label.Parent = b
	buttonRecipe.Bind(b, { tone = tone })
	buttonRecipe.Bind(label, { tone = tone })
	buttonRecipe.Bind(corner(b), { tone = tone })
	return b, label
end

-- A square tile carrying up to two initials: the icon stand-in.
local function initials(name: string): string
	local out = ""
	for word in string.gmatch(name, "%w+") do
		out ..= string.upper(string.sub(word, 1, 1))
		if #out == 2 then
			break
		end
	end
	return out
end

-- A `radius` rounds it by hand instead of by the button rule's corner.
local function tile(parent: Instance, name: string, order: number, px: number, letters: string, radius: number?): (Frame, TextLabel)
	local f = frame(parent, name, order, UDim2.fromOffset(px, px))
	f.BackgroundTransparency = 0
	buttonRecipe.Bind(f, { tone = "default" })
	if radius then
		corner(f, UDim.new(0, radius))
	else
		buttonRecipe.Bind(corner(f), { tone = "default" })
	end
	local glyph = Instance.new("TextLabel")
	glyph.Name = "Glyph"
	glyph.Size = UDim2.fromOffset(px, px)
	glyph.BackgroundTransparency = 1
	glyph.FontFace = BOLD
	glyph.TextSize = math.max(12, math.floor(px * 0.42))
	glyph.Text = letters
	glyph.Parent = f
	buttonRecipe.Bind(glyph, { tone = "default" })
	return f, glyph
end

-- A pill as wide as its text.
local function pill(parent: Instance, name: string, order: number, height: number, size: number): TextLabel
	local p = Instance.new("TextLabel")
	p.Name = name
	p.LayoutOrder = order
	p.BackgroundTransparency = 0
	p.FontFace = BOLD
	p.TextSize = size
	p.Size = UDim2.fromOffset(0, height)
	p.AutomaticSize = Enum.AutomaticSize.X
	p.Parent = parent
	pad(p, 8, 0, 8, 0)
	corner(p, UDim.new(0, height // 2))
	return p
end

-- A vertically scrolling column with a gap between its children.
local function scroller(parent: Instance, name: string, order: number): ScrollingFrame
	local s = Instance.new("ScrollingFrame")
	s.Name = name
	s.LayoutOrder = order
	s.Size = UDim2.fromScale(1, 0)
	s.BackgroundTransparency = 1
	s.BorderSizePixel = 0
	s.ScrollBarThickness = 4
	-- Children size against the canvas, so it tracks the frame's width and
	-- grows to the height its list lays out.
	s.CanvasSize = UDim2.fromScale(1, 0)
	s.AutomaticCanvasSize = Enum.AutomaticSize.Y
	s.Parent = parent
	grow(s)
	pad(s, 16, 0, 16, 16)
	list(s, Enum.FillDirection.Vertical, 8)
	return s
end

--------------------------------------------------------------------------
-- THE WINDOW: sidebar, divider, content.
--------------------------------------------------------------------------

local body = frame(root, "Body", 0, UDim2.fromScale(1, 1))
body.BackgroundTransparency = 0
windowRecipe.Bind(body, {})
list(body, Enum.FillDirection.Horizontal, 0)

local sidebar = frame(body, "Sidebar", 1, UDim2.fromScale(0.3, 1))
sidebar.BackgroundTransparency = 0
tabRecipe.Bind(sidebar, { state = "inactive" })
do
	local c = Instance.new("UISizeConstraint")
	c.MinSize = Vector2.new(124, 0)
	c.MaxSize = Vector2.new(200, math.huge)
	c.Parent = sidebar
end
pad(sidebar, 10, 14, 10, 10)
list(sidebar, Enum.FillDirection.Vertical, 4)

local divider = frame(body, "Divider", 2, UDim2.new(0, 1, 1, 0))
divider.BackgroundTransparency = 0
rowRecipe.Bind(divider, {})

local main = frame(body, "Main", 3, UDim2.fromScale(0, 1))
grow(main)

-- ALL THREE SECTIONS FILL `main`, ONE VISIBLE AT A TIME. Toggling `Visible`
-- rather than rebuilding keeps Store from re-running `Discover()` on every
-- visit.
local function section(name: string): Frame
	local f = frame(main, name, 0, UDim2.fromScale(1, 1))
	list(f, Enum.FillDirection.Vertical, 0)
	return f
end

local libraryTab = section("LibraryTab")
local storeTab = section("StoreTab")
local accountTab = section("AccountTab")

-- A section's title, a one line summary under it, and an optional button
-- centred beside them.
local function header(parent: Instance, title: string): (Frame, TextLabel)
	local h = frame(parent, "Header", 1, UDim2.fromScale(1, 0))
	h.AutomaticSize = Enum.AutomaticSize.Y
	pad(h, 16, 16, 16, 10)
	local l = list(h, Enum.FillDirection.Horizontal, 10)
	l.VerticalAlignment = Enum.VerticalAlignment.Center
	local titles = frame(h, "Titles", 1, UDim2.fromOffset(0, 0))
	titles.AutomaticSize = Enum.AutomaticSize.Y
	grow(titles)
	list(titles, Enum.FillDirection.Vertical, 4)
	line(titles, "Title", 1, 22, BOLD, "primary", 26).Text = title
	local subtitle = line(titles, "Subtitle", 2, 13, REGULAR, "secondary", 16)
	return h, subtitle
end

-- A wrapped status line under a section's header.
local function statusLine(parent: Instance, order: number): TextLabel
	local s = paragraph(parent, "Status", order, 13, "accent")
	pad(s, 16, 0, 16, 10)
	setMessage(s, "")
	return s
end

--------------------------------------------------------------------------
-- NAVIGATION. `Aether.Tabs`, controlled by `activeTab`, so the account card
-- at the sidebar's foot can open Account too. Tabs also answers Space,
-- Return and the arrow keys while an item holds keyboard selection.
--------------------------------------------------------------------------

type TabName = "library" | "store" | "account"

-- `Source` and `Action` are this dashboard's own vide: a source only
-- notifies the graph that created it, and only this vide's `create` can
-- apply its actions.
local activeTab = source("library" :: TabName)
local tabs = Tabs({
	Source = source,
	Action = vide.action,
	Tabs = { "library", "store", "account" } :: { TabName },
	Active = activeTab,
	OnActiveChanged = function(key: TabName)
		activeTab(key)
	end,
})

do
	local brand = frame(sidebar, "Brand", 1, UDim2.new(1, 0, 0, 34))
	pad(brand, 4, 0, 0, 0)
	local l = list(brand, Enum.FillDirection.Horizontal, 8)
	l.VerticalAlignment = Enum.VerticalAlignment.Center
	tile(brand, "Mark", 1, 24, "D")
	local name = line(brand, "Name", 2, 17, BOLD, "primary")
	name.Text = "Dew"
	name.Size = UDim2.fromOffset(0, 22)
	grow(name)
	frame(sidebar, "Gap", 2, UDim2.new(1, 0, 0, 10))
end

type NavItem = { button: TextButton, label: TextLabel, corner: UICorner, count: TextLabel? }

local function navItem(key: TabName, text: string, order: number, counted: boolean): NavItem
	local b = create("TextButton")(tabs.GetTab(key).GetRootProps({
		Name = `Tab{text}`,
		LayoutOrder = order,
		Size = UDim2.new(1, 0, 0, 36),
		BackgroundTransparency = 0,
		AutoButtonColor = false,
		Text = "",
		Parent = sidebar,
	})) :: TextButton
	pad(b, 10, 0, 8, 0)
	local l = list(b, Enum.FillDirection.Horizontal, 6)
	l.VerticalAlignment = Enum.VerticalAlignment.Center
	local label = line(b, "Label", 1, 15, BOLD, nil, 20)
	label.Text = text
	label.Size = UDim2.fromOffset(0, 20)
	grow(label)
	local count = if counted then pill(b, "Count", 2, 20, 12) else nil
	if count then
		count.Visible = false
	end
	return { button = b, label = label, corner = corner(b), count = count }
end

local nav: { [TabName]: NavItem } = {
	library = navItem("library", "Library", 3, true),
	store = navItem("store", "Store", 4, true),
	account = navItem("account", "Account", 5, false),
}

local function setCount(key: TabName, n: number?)
	local count = nav[key].count
	if count then
		count.Text = if n then tostring(n) else ""
		count.Visible = n ~= nil
	end
end

local spacer = frame(sidebar, "Spacer", 6, UDim2.fromScale(1, 0))
grow(spacer)

-- WHO IS SIGNED IN, at the foot of the sidebar on every section. Pressing it
-- opens Account. Filled in by `refreshAccount`.
local whoAvatar: Frame
local whoInitial: TextLabel
local whoPrimary: TextLabel
local whoSecondary: TextLabel

vide.root(function()
	local who = Aether.Pressable({
		Parent = sidebar,
		Name = "Who",
		LayoutOrder = 7,
		Size = UDim2.new(1, 0, 0, 48),
		BackgroundTransparency = 0,
		OnActivated = function()
			activeTab("account")
		end,
	}) :: Frame
	rowRecipe.Bind(who, {})
	rowRecipe.Bind(corner(who), {})
	pad(who, 8, 0, 8, 0)
	local l = list(who, Enum.FillDirection.Horizontal, 8)
	l.VerticalAlignment = Enum.VerticalAlignment.Center
	whoAvatar, whoInitial = tile(who, "Avatar", 1, 28, "", 14)
	local col = frame(who, "Text", 2, UDim2.fromOffset(0, 32))
	grow(col)
	list(col, Enum.FillDirection.Vertical, 2)
	whoPrimary = line(col, "Primary", 1, 13, BOLD, "primary", 16)
	whoSecondary = line(col, "Secondary", 2, 12, REGULAR, "secondary", 14)
end)

vide.root(function()
	effect(function()
		local current = activeTab()
		libraryTab.Visible = current == "library"
		storeTab.Visible = current == "store"
		accountTab.Visible = current == "account"
		for key, item in nav do
			local selection = { state = if key == current then "active" else "inactive" }
			tabRecipe.Bind(item.button, selection)
			tabRecipe.Bind(item.label, selection)
			tabRecipe.Bind(item.corner, selection)
			if item.count then
				chipRecipe.Bind(item.count, selection)
			end
		end
	end)
end)

--------------------------------------------------------------------------
-- ROWS, shared by Library and Store.
--
-- A row is icon, text block and buttons in one wrapping list, so a narrow
-- window moves the buttons onto their own line under the description. The
-- text block holds the name, the detail and the description, and grows to
-- the width the buttons leave.
--
-- FLAT ON PURPOSE: only the row, the text block and the description size
-- themselves (see `actionButton` for why that count matters).
--------------------------------------------------------------------------

type Row = { row: Frame, actions: Frame }

local function buildRow(
	parent: Instance,
	order: number,
	title: string,
	detail: string,
	description: string,
	buttonWidths: { number }
): Row
	local row = frame(parent, "Row", order, UDim2.fromScale(1, 0))
	row.AutomaticSize = Enum.AutomaticSize.Y
	row.BackgroundTransparency = 0
	rowRecipe.Bind(row, {})
	rowRecipe.Bind(corner(row), {})
	pad(row, ROW_GAP)
	local l = list(row, Enum.FillDirection.Horizontal, ROW_GAP)
	l.Wraps = true
	l.VerticalAlignment = Enum.VerticalAlignment.Center

	tile(row, "Icon", 1, ICON_SIZE, initials(title))

	local text = frame(row, "Text", 2, UDim2.fromOffset(0, 0))
	text.AutomaticSize = Enum.AutomaticSize.Y
	grow(text)
	atLeast(text, 120)
	list(text, Enum.FillDirection.Vertical, 2)
	line(text, "Name", 1, 15, BOLD, "primary", 19).Text = title
	line(text, "Detail", 2, 13, MONO, "secondary", 15).Text = detail
	local d = paragraph(text, "Description", 3, 13, "secondary")
	d.Text = description
	d.Visible = description ~= ""

	-- As wide as its buttons, which are fixed, so it never measures them.
	local width = 0
	for i, w in buttonWidths do
		width += w + if i > 1 then BUTTON_GAP else 0
	end
	local actions = frame(row, "Actions", 3, UDim2.fromOffset(width, BUTTON_HEIGHT))
	list(actions, Enum.FillDirection.Horizontal, BUTTON_GAP)

	return { row = row, actions = actions }
end

--------------------------------------------------------------------------
-- LIBRARY: every installed applet, with Enable/Disable and Uninstall.
-- `dew.Library`'s calls are synchronous, so a result is read the moment a
-- call returns.
--------------------------------------------------------------------------

local libraryHeader, librarySubtitle = header(libraryTab, "Library")

type Filter = "all" | "running" | "stopped"

-- WHICH ROWS SHOW: a second `Aether.Tabs`, controlled by `filter`, drives
-- the chips. Changing it rebuilds the rows.
local filter = source("all" :: Filter)
local rebuildLibraryRows: () -> number

local filters = Tabs({
	Source = source,
	Action = vide.action,
	Tabs = { "all", "running", "stopped" } :: { Filter },
	Active = filter,
	OnActiveChanged = function(key: Filter)
		filter(key)
		rebuildLibraryRows()
	end,
})

local chips: { [Filter]: { button: TextButton, corner: UICorner } } = {}
do
	local row = frame(libraryTab, "Filters", 2, UDim2.fromScale(1, 0))
	row.AutomaticSize = Enum.AutomaticSize.Y
	pad(row, 16, 0, 16, 10)
	list(row, Enum.FillDirection.Horizontal, 6).Wraps = true
	for i, key in { "all", "running", "stopped" } :: { Filter } do
		local b = create("TextButton")(filters.GetTab(key).GetRootProps({
			Name = `Filter{string.upper(string.sub(key, 1, 1))}{string.sub(key, 2)}`,
			LayoutOrder = i,
			Size = UDim2.fromOffset(0, 26),
			AutomaticSize = Enum.AutomaticSize.X,
			BackgroundTransparency = 0,
			AutoButtonColor = false,
			FontFace = BOLD,
			TextSize = 13,
			Parent = row,
		})) :: TextButton
		pad(b, 10, 0, 10, 0)
		chips[key] = { button = b, corner = corner(b, UDim.new(0, 13)) }
	end
end

local libraryStatus = statusLine(libraryTab, 3)
local libraryList = scroller(libraryTab, "List", 4)

local EMPTY_LIBRARY = "No applets installed yet. Find some in the Store."

-- ONE VIDE ROOT PER ROW, KEPT SO A REBUILD CAN TEAR IT DOWN FIRST.
-- `Aether.Pressable` and `Aether.Dialog` register cleanups with the scope
-- they are built in, and destroying an Instance does not run them, so
-- without this every rebuild would leave each button's PointerRouter
-- registration behind. The row's dialog is kept too: it is parented to
-- `root`, not the row, so destroying the row does not reach it.
local libraryRows: { [Frame]: { destroyScope: () -> (), dialog: Instance? } } = {}

local function clearLibraryRows()
	for _, child in libraryList:GetChildren() do
		if child:IsA("Frame") and child.Name == "Row" then
			local owned = libraryRows[child]
			if owned then
				-- The scope first: its cleanups unregister the buttons and the
				-- dialog while their Instances still exist.
				owned.destroyScope()
				if owned.dialog then
					owned.dialog:Destroy()
				end
				libraryRows[child] = nil
			end
			child:Destroy()
		end
	end
end

--[[
	THE TOGGLE IS KEYED ON `running`, NOT `enabled`. `running` comes from the
	host's coordinator, `enabled` from `installed.json`. An applet can be
	enabled and not running (closed by hand, or crashed); keyed on `enabled`
	its button would read "Disable" and turn it off instead of starting it.
	Keyed on `running`, off always reads "Enable" and starts it, and on
	always reads "Disable" and stops it.
]]
local function buildLibraryRow(entry: LibraryEntry, order: number)
	local owned: { destroyScope: () -> (), dialog: Instance? } = { destroyScope = function() end }
	local destroyScope, row = vide.root(function(): Frame
		local built = buildRow(
			libraryList,
			order,
			entry.name,
			entry.id,
			if entry.description == "" then entry.id else entry.description,
			if entry.bundled then { NARROW } else { NARROW, WIDE }
		)

		-- Neither handler rebuilds the list: `library.OnChange` does, for
		-- every change whatever its source. A handler only reports its result.
		actionButton(built.actions, "Toggle", 1, if entry.running then "Disable" else "Enable", "default", NARROW, function()
			local result = library.SetEnabled(entry.id, not entry.running)
			setMessage(
				libraryStatus,
				if result.ok then `{if entry.running then "Disabled" else "Enabled"} '{entry.id}'` else result.error
			)
		end)

		if entry.bundled then
			return built.row
		end

		-- Uninstall asks first. The dialog is built in this row's scope, so
		-- the row's teardown also tears down its machine.
		local uninstallDialog: any = nil
		local uninstallButton: Frame
		uninstallButton = actionButton(built.actions, "Uninstall", 2, "Uninstall", "danger", WIDE, function()
			-- Handed the trigger, so the focus trap restores focus to it on close.
			uninstallDialog.Open(uninstallButton)
		end)

		local dialogFrame = Aether.Dialog({
			Id = `uninstall-{entry.id}`,
			Name = "UninstallConfirm",
			Title = "Uninstall applet?",
			Size = UDim2.fromOffset(320, 150),
			OutDialog = function(d: any)
				uninstallDialog = d
			end,
			Children = function(dialog: any)
				-- Fills the dialog's surface, padded clear of its title bar.
				local content = Instance.new("Frame")
				content.Name = "Content"
				content.Size = UDim2.fromScale(1, 1)
				content.BackgroundTransparency = 1
				pad(content, 16, 48, 16, 16)
				list(content, Enum.FillDirection.Vertical, 8)

				local message = paragraph(content, "Message", 1, 13, "primary")
				message.Text = `This removes '{entry.name}' from this machine.`
				local push = frame(content, "Spacer", 2, UDim2.fromScale(1, 0))
				grow(push)

				local buttons = frame(content, "Buttons", 3, UDim2.new(1, 0, 0, BUTTON_HEIGHT))
				local bl = list(buttons, Enum.FillDirection.Horizontal, 8)
				bl.HorizontalAlignment = Enum.HorizontalAlignment.Right
				actionButton(buttons, "Cancel", 1, "Cancel", "default", NARROW, function()
					dialog.Close()
				end)
				-- `Close()` runs first, while this row's scope is alive, so the
				-- screen stack closes cleanly. `Uninstall` then fires `OnChange`,
				-- and the rebuild destroys this dialog along with its row.
				actionButton(buttons, "Confirm", 2, "Uninstall", "danger", WIDE, function()
					dialog.Close()
					local result = library.Uninstall(entry.id)
					setMessage(libraryStatus, if result.ok then `Uninstalled '{entry.id}'` else result.error)
				end)
				return content
			end,
		})
		-- `Aether.Dialog` does not parent itself. Under `root`, beside the
		-- body, so its backdrop covers the whole window.
		dialogFrame.Parent = root
		owned.dialog = dialogFrame

		return built.row
	end)
	owned.destroyScope = destroyScope
	libraryRows[row] = owned
end

local function libraryCounts(entries: { LibraryEntry }): (number, number)
	local running = 0
	for _, entry in entries do
		if entry.running then
			running += 1
		end
	end
	return #entries, running
end

-- Rebuilds the rows for the current filter, and every count that depends on
-- the list. Returns how many applets are installed.
rebuildLibraryRows = function(): number
	local entries = library.List() :: { LibraryEntry }
	local total, running = libraryCounts(entries)
	librarySubtitle.Text = `{total} installed, {running} running`
	setCount("library", total)

	local current = filter()
	local labels: { [Filter]: string } = {
		all = `All {total}`,
		running = `Running {running}`,
		stopped = `Stopped {total - running}`,
	}
	for key, chip in chips do
		chip.button.Text = labels[key]
		local selection = { state = if key == current then "active" else "inactive" }
		chipRecipe.Bind(chip.button, selection)
		chipRecipe.Bind(chip.corner, selection)
	end

	clearLibraryRows()
	for i, entry in entries do
		if current == "all" or entry.running == (current == "running") then
			buildLibraryRow(entry, i)
		end
	end
	return total
end

local function refreshLibrary()
	local count = rebuildLibraryRows()
	setMessage(libraryStatus, if count == 0 then EMPTY_LIBRARY else "")
end

-- The one place the rows rebuild besides Refresh and the first load. Fires
-- for a change from any source: a button here, a crash, or `dew install`
-- run in a terminal. Leaves the status alone unless the list is empty, so a
-- button's own result message survives the rebuild it caused.
library.OnChange(function()
	local count = rebuildLibraryRows()
	if count == 0 then
		setMessage(libraryStatus, EMPTY_LIBRARY)
	end
end)

--------------------------------------------------------------------------
-- STORE: what `Discover` returns, each with Install. `Discover` and
-- `Install` call back once, when their answer lands.
--------------------------------------------------------------------------

local storeHeader, storeSubtitle = header(storeTab, "Store")
local storeStatus = statusLine(storeTab, 3)
local storeList = scroller(storeTab, "List", 4)

-- State first: `buildStoreRow` closes over `onInstalled`, and a function
-- captures a local only if it is declared before the function is.
local packages = source(nil :: { Package }?)

local function onDiscovered(discovered)
	if discovered.ok then
		packages(discovered.packages)
		storeSubtitle.Text = `{#discovered.packages} packages from the marketplace`
		setCount("store", #discovered.packages)
		setMessage(storeStatus, if #discovered.packages == 0 then "No public packages yet" else "")
	else
		setMessage(storeStatus, discovered.error)
	end
end

local function onInstalled(installed)
	setMessage(storeStatus, if installed.ok then `Installed '{installed.id}'` else installed.error)
	-- A new install is a Library entry, so Library refreshes too.
	if installed.ok then
		refreshLibrary()
	end
end

-- Built only from the effect below, which is a live vide scope while it
-- runs: each Install button's cleanup registers with that effect and runs
-- before its next run, so no per-row scope is needed here.
local function buildStoreRow(pkg: Package, order: number)
	local built = buildRow(
		storeList,
		order,
		pkg.appletId,
		`@{pkg.ownerUserId}/{pkg.appletId}`,
		`Published {pkg.uploadedAt}`,
		{ NARROW }
	)
	actionButton(built.actions, "Install", 1, "Install", "success", NARROW, function()
		setMessage(storeStatus, `Installing {pkg.appletId}...`)
		marketplace.Install(pkg.ownerUserId, pkg.appletId, onInstalled)
	end)
end

local function discover(message: string)
	setMessage(storeStatus, message)
	marketplace.Discover(onDiscovered)
end

-- The source is the only state; this effect is the only thing that reacts.
vide.root(function()
	effect(function()
		local current = packages()
		for _, child in storeList:GetChildren() do
			if child:IsA("Frame") and child.Name == "Row" then
				child:Destroy()
			end
		end
		if current == nil then
			return
		end
		for i, pkg in current do
			buildStoreRow(pkg, i)
		end
	end)
end)

--------------------------------------------------------------------------
-- ACCOUNT: who is signed in, signing out, and signing in.
--
-- KNOWN LIMITATION: the password field is a plain TextBox and is not
-- masked. The DataModel's TextBox has no masking property, and doing it by
-- hand means intercepting every keystroke.
--------------------------------------------------------------------------

local _, accountSubtitle = header(accountTab, "Account")
local accountList = scroller(accountTab, "Sections", 2)

local function card(order: number, heading: string): Frame
	local c = frame(accountList, "Section", order, UDim2.fromScale(1, 0))
	c.AutomaticSize = Enum.AutomaticSize.Y
	c.BackgroundTransparency = 0
	rowRecipe.Bind(c, {})
	rowRecipe.Bind(corner(c), {})
	pad(c, 14)
	list(c, Enum.FillDirection.Vertical, 10)
	line(c, "Heading", 0, 12, BOLD, "secondary", 15).Text = heading
	return c
end

-- A form line: a label 84 wide beside a value that takes the rest and is
-- never narrower than 170. Narrower than that, the value wraps under the
-- label. Both are as tall as a field and neither is AutomaticSize, so the
-- line's own alignment never has to place a growing child.
local function formLine(
	parent: Instance,
	order: number,
	labelName: string,
	labelText: string,
	height: number
): (Frame, TextLabel)
	local f = frame(parent, "Line", order, UDim2.fromScale(1, 0))
	f.AutomaticSize = Enum.AutomaticSize.Y
	list(f, Enum.FillDirection.Horizontal, 10).Wraps = true
	local key = line(f, labelName, 1, 14, REGULAR, "secondary", height)
	key.Text = labelText
	key.Size = UDim2.fromOffset(84, height)
	return f, key
end

local function fieldSlot(child: GuiObject, height: number)
	child.LayoutOrder = 2
	child.Size = UDim2.fromOffset(0, height)
	grow(child)
	atLeast(child, 170)
end

-- SIGNED IN: the profile, and the session with Sign out. Read-only lines
-- are one text line tall; there is no field to line up with.
local VALUE_HEIGHT = 22
local profileSection = card(1, "PROFILE")

local function profileLine(order: number, name: string, label: string, size: number, font: Font): TextLabel
	local value = line(formLine(profileSection, order, `{name}Label`, label, VALUE_HEIGHT), `{name}Value`, 2, size, font, "primary")
	fieldSlot(value, VALUE_HEIGHT)
	return value
end

local profileEmail = profileLine(1, "Email", "Email", 14, BOLD)
local profileUserId = profileLine(2, "UserId", "User ID", 13, MONO)
local profileLibrary = profileLine(3, "Library", "Library", 14, REGULAR)

local sessionSection = card(2, "SESSION")
local accountStatus = paragraph(sessionSection, "Status", 1, 13, "accent")
local sessionActions = frame(sessionSection, "Actions", 2, UDim2.new(1, 0, 0, BUTTON_HEIGHT))
list(sessionActions, Enum.FillDirection.Horizontal, 8)

-- SIGNED OUT: a sign-in form. Both states exist at once and `refreshAccount`
-- shows one.
local signInSection = card(3, "SIGN IN")
paragraph(signInSection, "Hint", 1, 13, "primary").Text =
	"Sign in with your Dew account to install applets from the Store and publish your own."

local function textField(parent: Instance, name: string, placeholder: string): TextBox
	local box = Instance.new("TextBox")
	box.Name = name
	box.PlaceholderText = placeholder
	box.FontFace = REGULAR
	box.TextSize = 14
	box.TextXAlignment = Enum.TextXAlignment.Left
	box.ClearTextOnFocus = false
	-- Text longer than the field is clipped at its edge.
	box.ClipsDescendants = true
	box.Text = ""
	box.Parent = parent
	fieldSlot(box, FIELD_HEIGHT)
	pad(box, 10, 0, 10, 0)
	textInputRecipe.Bind(box, {})
	textInputRecipe.Bind(corner(box), {})
	local stroke = Instance.new("UIStroke")
	-- Around the field, not the glyphs it holds.
	stroke.ApplyStrokeMode = Enum.ApplyStrokeMode.Border
	stroke.Parent = box
	textInputRecipe.Bind(stroke, {})
	return box
end

local emailLine, emailLabel = formLine(signInSection, 2, "EmailLabel", "Email", FIELD_HEIGHT)
local emailBox = textField(emailLine, "Email", "you@example.com")
local passwordLine, passwordLabel = formLine(signInSection, 3, "PasswordLabel", "Password", FIELD_HEIGHT)
local passwordBox = textField(passwordLine, "Password", "Password")
local signInActions = frame(signInSection, "Actions", 4, UDim2.new(1, 0, 0, BUTTON_HEIGHT))
list(signInActions, Enum.FillDirection.Horizontal, 8)
local signInError = paragraph(signInSection, "SignInError", 5, 13, "error")
setMessage(signInError, "")

local signOutButton: Frame
local signInButton: Frame
local signInLabel: TextLabel
local refreshAccount: () -> ()

-- A second click while a sign-in is outstanding is ignored, and the button
-- says one is under way.
local signInInFlight = false

-- Neither button is ever rebuilt, so one scope whose destructor never runs
-- is enough: the window owns this panel's lifetime.
vide.root(function()
	signOutButton = actionButton(sessionActions, "SignOut", 1, "Sign out", "danger", NARROW, function()
		account.SignOut()
		refreshAccount()
	end)

	signInButton, signInLabel = actionButton(signInActions, "SignIn", 1, "Sign in", "default", 112, function()
		if signInInFlight then
			return
		end
		local email = emailBox.Text
		local password = passwordBox.Text
		if email == "" or password == "" then
			setMessage(signInError, "Enter an email and password")
			return
		end

		signInInFlight = true
		signInLabel.Text = "Signing in..."
		setMessage(signInError, "")

		account.SignIn(email, password, function(result)
			signInInFlight = false
			signInLabel.Text = "Sign in"
			if result.ok then
				passwordBox.Text = ""
				refreshAccount()
			else
				setMessage(signInError, result.error)
			end
		end)
	end)
end)

refreshAccount = function()
	local session = account.Whoami() :: Session?
	local signedIn = session ~= nil

	profileSection.Visible = signedIn
	sessionSection.Visible = signedIn
	accountStatus.Visible = signedIn
	signOutButton.Visible = signedIn

	signInSection.Visible = not signedIn
	emailLabel.Visible = not signedIn
	emailBox.Visible = not signedIn
	passwordLabel.Visible = not signedIn
	passwordBox.Visible = not signedIn
	signInButton.Visible = not signedIn
	if signedIn then
		setMessage(signInError, "")
	end

	if session then
		accountSubtitle.Text = "Your Dew account on this machine"
		accountStatus.Text = `Signed in as {session.email}`
		profileEmail.Text = session.email
		profileUserId.Text = session.userId or ""
		local total, running = libraryCounts(library.List() :: { LibraryEntry })
		profileLibrary.Text = `{total} applets, {running} running`
		whoPrimary.Text = session.email
		whoSecondary.Text = "Signed in"
		textRecipe.Bind(whoSecondary, { tone = "secondary" })
		whoInitial.Text = string.upper(string.sub(session.email, 1, 1))
	else
		accountSubtitle.Text = "Not signed in"
		whoPrimary.Text = "Not signed in"
		whoSecondary.Text = "Sign in"
		textRecipe.Bind(whoSecondary, { tone = "accent" })
		whoInitial.Text = ""
	end
end

--------------------------------------------------------------------------
-- REFRESH, AND THE FIRST LOAD.
--------------------------------------------------------------------------

vide.root(function()
	actionButton(libraryHeader, "Refresh", 2, "Refresh", "default", NARROW, refreshLibrary)
	actionButton(storeHeader, "Refresh", 2, "Refresh", "default", NARROW, function()
		discover("Refreshing...")
	end)
end)

refreshAccount()
discover("Loading...")
refreshLibrary()

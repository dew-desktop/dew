--!strict
--[[
	The Dew dashboard -- Library, Store, and Account in one window
	(milestone 25), grown from milestone 23's marketplace-only browser. The
	Store tab below is that original browser, moved under a tab and
	otherwise untouched; Library and Account are new, built on
	`dew.Library` and `dew.Account` (ADR-017/ADR-018). `manage.rs`'s Win32
	window still owns sign-IN itself -- this reads and acts on state that
	already exists, the same boundary `dew.Account` draws.

	Bundled with the host rather than installed like a marketplace package
	-- `dew.toml` asks for `discover`, `install`, `library` and `auth`,
	`Capability::Host` permissions no ordinary applet can ever be granted.
]]

local vide = require("./roblox_packages/vide")
local source = vide.source
local effect = vide.effect

local window = assert(desktop.Window, "dew.toml must grant `window`")
local marketplace = assert(dew.Marketplace, "dew.toml must grant `discover` and `install`")
local library = assert(dew.Library, "dew.toml must grant `library`")
local account = assert(dew.Account, "dew.toml must grant `auth`")

-- TALLER BY EXACTLY THE TAB BAR'S OWN HEIGHT (see `TAB_BAR_HEIGHT` below),
-- so each tab's content frame keeps the 420x520 the Store tab's layout was
-- always built against, rather than that layout having to shrink to make
-- room for tabs above it.
local root = window({ title = "Dew Dashboard", width = 420, height = 560 })

type Package = { ownerUserId: string, appletId: string, uploadedAt: string }
type LibraryEntry = { id: string, name: string, description: string, enabled: boolean, running: boolean }

local function label(
	parent: Instance,
	name: string,
	position: UDim2,
	size: UDim2,
	textSize: number,
	colour: Color3
): TextLabel
	local text = Instance.new("TextLabel")
	text.Name = name
	text.Position = position
	text.Size = size
	text.BackgroundTransparency = 1
	text.TextSize = textSize
	text.TextColor3 = colour
	text.TextXAlignment = Enum.TextXAlignment.Left
	text.Parent = parent
	return text
end

local function button(parent: Instance, name: string, position: UDim2, size: UDim2, text: string, bg: Color3, fg: Color3): TextButton
	local b = Instance.new("TextButton")
	b.Name = name
	b.Position = position
	b.Size = size
	b.BackgroundColor3 = bg
	b.TextColor3 = fg
	b.TextSize = 12
	b.Text = text
	b.Parent = parent
	return b
end

local body = Instance.new("Frame")
body.Name = "Body"
body.Size = UDim2.new(1, 0, 1, 0)
body.BackgroundColor3 = Color3.fromRGB(18, 20, 26)
body.Parent = root

--------------------------------------------------------------------------
-- THE TAB BAR. Three buttons, one active at a time; `activeTab` is the
-- only state, and the effect below is the only thing that reacts to it --
-- the same split the Store tab's own `packages` source already uses.
--------------------------------------------------------------------------

local TAB_BAR_HEIGHT = 40

type TabName = "library" | "store" | "account"
local activeTab = source("library" :: TabName)

local ACTIVE_TAB_COLOR = Color3.fromRGB(48, 66, 94)
local INACTIVE_TAB_COLOR = Color3.fromRGB(26, 29, 38)

local tabBar = Instance.new("Frame")
tabBar.Name = "TabBar"
tabBar.Position = UDim2.new(0, 0, 0, 0)
tabBar.Size = UDim2.new(1, 0, 0, TAB_BAR_HEIGHT)
tabBar.BackgroundTransparency = 1
tabBar.Parent = body

local TAB_WIDTH = 132

local function tabButton(name: TabName, text: string, x: number): TextButton
	local b = button(
		tabBar,
		`Tab{text}`,
		UDim2.new(0, x, 0, 8),
		UDim2.new(0, TAB_WIDTH - 4, 0, 28),
		text,
		INACTIVE_TAB_COLOR,
		Color3.fromRGB(206, 222, 246)
	)
	b.Activated:Connect(function()
		activeTab(name)
	end)
	return b
end

local libraryTabButton = tabButton("library", "Library", 8)
local storeTabButton = tabButton("store", "Store", 8 + TAB_WIDTH)
local accountTabButton = tabButton("account", "Account", 8 + TAB_WIDTH * 2)

-- CONTENT FRAMES, ALL THE SAME RECT, ONE VISIBLE AT A TIME -- toggling
-- `Visible` rather than destroying and rebuilding a tab's tree every
-- switch, which would re-run `Discover()` on every visit to Store.
local function contentFrame(name: string): Frame
	local f = Instance.new("Frame")
	f.Name = name
	f.Position = UDim2.new(0, 0, 0, TAB_BAR_HEIGHT)
	f.Size = UDim2.new(1, 0, 1, -TAB_BAR_HEIGHT)
	f.BackgroundTransparency = 1
	f.Parent = body
	return f
end

local libraryTab = contentFrame("LibraryTab")
local storeTab = contentFrame("StoreTab")
local accountTab = contentFrame("AccountTab")

vide.root(function()
	effect(function()
		local tab = activeTab()
		libraryTab.Visible = tab == "library"
		storeTab.Visible = tab == "store"
		accountTab.Visible = tab == "account"

		local highlight = function(b: TextButton, active: boolean)
			b.BackgroundColor3 = if active then ACTIVE_TAB_COLOR else INACTIVE_TAB_COLOR
		end
		highlight(libraryTabButton, tab == "library")
		highlight(storeTabButton, tab == "store")
		highlight(accountTabButton, tab == "account")
	end)
end)

--------------------------------------------------------------------------
-- LIBRARY: every installed applet, with a way to launch, uninstall, and
-- enable/disable it. `dew.Library`'s four calls are all synchronous (see
-- `capabilities.rs`), so there is nothing here to poll -- a call's result
-- is read the instant it returns, the same as any ordinary function.
--------------------------------------------------------------------------

label(libraryTab, "Title", UDim2.new(0, 16, 0, 12), UDim2.new(1, -112, 0, 24), 18, Color3.fromRGB(233, 239, 249)).Text =
	"Library"

local libraryRefreshButton = button(
	libraryTab,
	"Refresh",
	UDim2.new(1, -96, 0, 12),
	UDim2.new(0, 80, 0, 24),
	"Refresh",
	Color3.fromRGB(38, 52, 74),
	Color3.fromRGB(206, 222, 246)
)

local libraryStatus =
	label(libraryTab, "Status", UDim2.new(0, 16, 0, 44), UDim2.new(1, -32, 0, 40), 12, Color3.fromRGB(146, 162, 188))
libraryStatus.TextWrapped = true
libraryStatus.TextYAlignment = Enum.TextYAlignment.Top

local libraryList = Instance.new("ScrollingFrame")
libraryList.Name = "List"
libraryList.Position = UDim2.new(0, 16, 0, 92)
libraryList.Size = UDim2.new(1, -32, 1, -108)
libraryList.BackgroundTransparency = 1
libraryList.CanvasSize = UDim2.new(1, 0, 0, 0)
libraryList.AutomaticCanvasSize = Enum.AutomaticSize.Y
libraryList.Parent = libraryTab

local libraryListLayout = Instance.new("UIListLayout")
libraryListLayout.Padding = UDim.new(0, 8)
libraryListLayout.Parent = libraryList

local LIBRARY_ROW_HEIGHT = 84

local function clearLibraryRows()
	for _, child in ipairs(libraryList:GetChildren()) do
		if child:IsA("Frame") and child.Name == "Row" then
			child:Destroy()
		end
	end
end

-- FORWARD-DECLARED: `buildLibraryRow`'s own buttons call `rebuildLibraryRows`
-- to rebuild the list after each action, and `rebuildLibraryRows` calls
-- `buildLibraryRow` for each entry -- the two close over each other.
--
-- SEPARATE FROM `refreshLibrary` BELOW ON PURPOSE. A row's own action
-- already wrote its own result into `libraryStatus.Text` (`Enabled 'x'`,
-- an error, ...); rebuilding the row list must not then overwrite that
-- with the empty-store notice, which is `refreshLibrary`'s job for the
-- Refresh button and the initial load, neither of which has a result of
-- its own to preserve.
local rebuildLibraryRows: () -> number

--[[
	DESIGN NOTE, REVISIT IF THIS STOPS FEELING RIGHT: only Enable/Disable
	and Uninstall are shown here -- there used to be a third "Launch"
	button, removed after it turned out to just be confusing rather than
	load-bearing.

	`enabled` (persisted in `installed.json`, "load this automatically at
	the next service start") and "is this actually running right now" are
	genuinely two different axes -- `dew.Library.SetEnabled` did not always
	touch the second one. It used to be a disk write ONLY: an applet could
	be enabled and running, enabled and NOT running (closed by hand, or
	before this session's own live-toggle fix, simply "not yet restarted"),
	or disabled while still running until the next restart. "Launch" was
	the one way to force it running right now without touching the
	persisted bit, which is why it existed as its own button next to
	Enable/Disable rather than folded into it.

	Once `SetEnabled` started live-loading/unloading (so the two axes stay
	in sync the instant you click), the only case Launch still covered on
	its own was an applet still `enabled` but no longer running -- closed
	by hand, or crashed -- where the toggle would read "Disable" (since
	`enabled` says so) and clicking it would turn the thing OFF, not bring
	it back. That is confusing on its face: two controls that mostly do the
	same thing, and the one time they diverge, the more prominent one
	(Enable/Disable) does the wrong thing.

	THE FIX: `LibraryEntry.running` (from `dew.Library.List`, backed by
	`coordinator::running_snapshot()`, not `installed.json`) is what this
	toggle's label and action are keyed on now, not `entry.enabled`. Off
	(not running) always reads "Enable" and always starts it; on (running)
	always reads "Disable" and always stops it. `SetEnabled(id, true)` on
	an enabled-but-stopped applet is exactly the old `Launch`'s job --
	start it without changing a bit that was already correct -- so nothing
	is lost, and the enabled-but-stopped case now shows the right label
	instead of the wrong one.
]]
local function buildLibraryRow(entry: LibraryEntry)
	local row = Instance.new("Frame")
	row.Name = "Row"
	row.Size = UDim2.new(1, 0, 0, LIBRARY_ROW_HEIGHT)
	row.BackgroundColor3 = Color3.fromRGB(28, 31, 40)
	row.Parent = libraryList

	local corner = Instance.new("UICorner")
	corner.CornerRadius = UDim.new(0, 8)
	corner.Parent = row

	label(row, "Name", UDim2.new(0, 12, 0, 8), UDim2.new(1, -24, 0, 18), 14, Color3.fromRGB(233, 239, 249)).Text =
		entry.name
	local description =
		label(row, "Description", UDim2.new(0, 12, 0, 28), UDim2.new(1, -24, 0, 16), 11, Color3.fromRGB(146, 162, 188))
	description.Text = if entry.description == "" then entry.id else entry.description

	local toggleButton = button(
		row,
		"Toggle",
		UDim2.new(0, 12, 0, 52),
		UDim2.new(0, 100, 0, 24),
		if entry.running then "Disable" else "Enable",
		Color3.fromRGB(38, 52, 74),
		Color3.fromRGB(206, 222, 246)
	)
	local uninstallButton = button(
		row,
		"Uninstall",
		UDim2.new(0, 124, 0, 52),
		UDim2.new(0, 88, 0, 24),
		"Uninstall",
		Color3.fromRGB(90, 48, 48),
		Color3.fromRGB(240, 214, 214)
	)

	toggleButton.Activated:Connect(function()
		local result = library.SetEnabled(entry.id, not entry.running)
		libraryStatus.Text = if result.ok
			then `{if entry.running then "Disabled" else "Enabled"} '{entry.id}'`
			else result.error
		rebuildLibraryRows()
	end)

	uninstallButton.Activated:Connect(function()
		local result = library.Uninstall(entry.id)
		libraryStatus.Text = if result.ok then `Uninstalled '{entry.id}'` else result.error
		rebuildLibraryRows()
	end)
end

rebuildLibraryRows = function(): number
	local entries = library.List() :: { LibraryEntry }
	clearLibraryRows()
	for _, entry in ipairs(entries) do
		buildLibraryRow(entry)
	end
	return #entries
end

local function refreshLibrary()
	local count = rebuildLibraryRows()
	libraryStatus.Text = if count == 0 then "No applets installed yet -- see the Store tab" else ""
end

libraryRefreshButton.Activated:Connect(refreshLibrary)

--------------------------------------------------------------------------
-- STORE: milestone 23's marketplace browser, unchanged beyond parenting
-- into `storeTab` instead of `body` directly.
--------------------------------------------------------------------------

label(storeTab, "Title", UDim2.new(0, 16, 0, 12), UDim2.new(1, -112, 0, 24), 18, Color3.fromRGB(233, 239, 249)).Text =
	"Marketplace"

local refreshButton = button(
	storeTab,
	"Refresh",
	UDim2.new(1, -96, 0, 12),
	UDim2.new(0, 80, 0, 24),
	"Refresh",
	Color3.fromRGB(38, 52, 74),
	Color3.fromRGB(206, 222, 246)
)

-- WRAPPED, AND GIVEN ROOM FOR MORE THAN ONE LINE: a network or auth error
-- (see `platform::describe_auth_error`) is a full sentence, not a label,
-- and an unwrapped one just runs off the edge of the window rather than
-- being clipped -- which reads as "blank" at a glance, not as an error.
local status =
	label(storeTab, "Status", UDim2.new(0, 16, 0, 44), UDim2.new(1, -32, 0, 40), 12, Color3.fromRGB(146, 162, 188))
status.TextWrapped = true
status.TextYAlignment = Enum.TextYAlignment.Top

local list = Instance.new("ScrollingFrame")
list.Name = "List"
list.Position = UDim2.new(0, 16, 0, 92)
list.Size = UDim2.new(1, -32, 1, -108)
list.BackgroundTransparency = 1
-- A ROW IS SIZED AGAINST THE CANVAS, NOT THE FRAME'S OWN VISIBLE BOX --
-- `render.rs` lays every child out inside `CanvasSize`'s box, and that
-- defaults to zero on both axes. Without this, `UDim2.new(1, 0, 0, ...)`
-- on a row resolves its width against a zero-wide canvas: every row
-- exists, at zero width, which is indistinguishable from nothing at all.
-- Width tracks the frame itself; height grows to fit whatever the list
-- layout below actually lays out.
list.CanvasSize = UDim2.new(1, 0, 0, 0)
list.AutomaticCanvasSize = Enum.AutomaticSize.Y
list.Parent = storeTab

local listLayout = Instance.new("UIListLayout")
listLayout.Padding = UDim.new(0, 8)
listLayout.Parent = list

local ROW_HEIGHT = 64

-- STATE FIRST, EVERYTHING ELSE AFTER. `buildRow` below closes over
-- `awaitingInstall`; declaring it after `buildRow` would make that a
-- capture of a global instead of this local, since a Luau chunk resolves
-- an upvalue at the point a function is DEFINED, not where it is called.
local packages = source(nil :: { Package }?)

-- AWAITING FLAGS, NOT A COMPARISON AGAINST THE LAST POLLED TABLE.
-- `Discovered()`/`Installed()` build a fresh Lua table every call (see
-- `capabilities.rs`), so two reads of an unchanged answer are never `==`
-- to each other -- polling every frame and writing straight into
-- `packages` would rebuild the whole row list every frame, forever, once
-- one result ever landed. A flag set at trigger time and cleared the one
-- time its answer is actually consumed is what keeps each result read
-- exactly once.
local awaitingDiscover = false
local awaitingInstall = false

local function clearRows()
	for _, child in ipairs(list:GetChildren()) do
		if child:IsA("Frame") and child.Name == "Row" then
			child:Destroy()
		end
	end
end

local function buildRow(pkg: Package)
	local row = Instance.new("Frame")
	row.Name = "Row"
	row.Size = UDim2.new(1, 0, 0, ROW_HEIGHT)
	row.BackgroundColor3 = Color3.fromRGB(28, 31, 40)
	row.Parent = list

	local corner = Instance.new("UICorner")
	corner.CornerRadius = UDim.new(0, 8)
	corner.Parent = row

	label(row, "Name", UDim2.new(0, 12, 0, 8), UDim2.new(1, -100, 0, 18), 14, Color3.fromRGB(233, 239, 249)).Text =
		`@{pkg.ownerUserId}/{pkg.appletId}`
	label(row, "Meta", UDim2.new(0, 12, 0, 30), UDim2.new(1, -100, 0, 16), 11, Color3.fromRGB(146, 162, 188)).Text =
		`published {pkg.uploadedAt}`

	local install = button(
		row,
		"Install",
		UDim2.new(1, -84, 0.5, -12),
		UDim2.new(0, 72, 0, 24),
		"Install",
		Color3.fromRGB(48, 90, 64),
		Color3.fromRGB(214, 240, 224)
	)

	install.Activated:Connect(function()
		status.Text = `Installing {pkg.appletId}...`
		awaitingInstall = true
		marketplace.Install(pkg.ownerUserId, pkg.appletId)
	end)
end

-- THE SOURCE IS THE ONLY STATE; THE EFFECT IS THE ONLY PLACE THAT REACTS,
-- the same split `widget-behaviors` already uses. `desktop.Clock.OnFrame`
-- below only senses and pushes; rebuilding the row list is this effect's
-- job alone.
vide.root(function()
	effect(function()
		local current = packages()
		clearRows()
		if current == nil then
			return
		end
		for _, pkg in ipairs(current) do
			buildRow(pkg)
		end
	end)
end)

desktop.Clock.OnFrame(function()
	if awaitingDiscover then
		local discovered = marketplace.Discovered()
		if discovered ~= nil then
			awaitingDiscover = false
			if discovered.ok then
				packages(discovered.packages)
				status.Text = if #discovered.packages == 0 then "No public packages yet" else ""
			else
				status.Text = discovered.error
			end
		end
	end

	if awaitingInstall then
		local installed = marketplace.Installed()
		if installed ~= nil then
			awaitingInstall = false
			status.Text = if installed.ok then `Installed '{installed.id}'` else installed.error
			-- THE LIBRARY TAB IS WHAT SHOWS AN INSTALL'S RESULT, since a
			-- newly installed applet is a Library entry, not a Store one --
			-- refreshed here too so switching tabs right after an install
			-- shows it without a manual Refresh click.
			if installed.ok then
				refreshLibrary()
			end
		end
	end
end)

refreshButton.Activated:Connect(function()
	status.Text = "Refreshing..."
	awaitingDiscover = true
	marketplace.Discover()
end)

status.Text = "Loading..."
awaitingDiscover = true
marketplace.Discover()

--------------------------------------------------------------------------
-- ACCOUNT: who is signed in, and a way to sign out. Sign-IN stays in
-- `manage.rs`'s Win32 window / `dew login` for this milestone -- see the
-- vision overview -- so a signed-out state points there rather than
-- offering a form this tab cannot back.
--------------------------------------------------------------------------

label(accountTab, "Title", UDim2.new(0, 16, 0, 12), UDim2.new(1, -32, 0, 24), 18, Color3.fromRGB(233, 239, 249)).Text =
	"Account"

local accountStatus =
	label(accountTab, "Status", UDim2.new(0, 16, 0, 52), UDim2.new(1, -32, 0, 60), 13, Color3.fromRGB(206, 222, 246))
accountStatus.TextWrapped = true
accountStatus.TextYAlignment = Enum.TextYAlignment.Top

local signOutButton = button(
	accountTab,
	"SignOut",
	UDim2.new(0, 16, 0, 116),
	UDim2.new(0, 96, 0, 28),
	"Sign out",
	Color3.fromRGB(90, 48, 48),
	Color3.fromRGB(240, 214, 214)
)

local function refreshAccount()
	local session = account.Whoami()
	if session then
		accountStatus.Text = `Signed in as {session.email}`
		signOutButton.Visible = true
	else
		accountStatus.Text = "Not signed in. Run `dew login` from a terminal to sign in."
		signOutButton.Visible = false
	end
end

signOutButton.Activated:Connect(function()
	account.SignOut()
	refreshAccount()
end)

refreshAccount()

--------------------------------------------------------------------------
-- INITIAL STATE, for the tab that opens active by default.
--------------------------------------------------------------------------

refreshLibrary()

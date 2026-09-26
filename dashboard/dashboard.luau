--!strict
--[[
	The Dew dashboard -- Library, Store, and Account in one window
	(milestone 25), grown from milestone 23's marketplace-only browser. The
	Store tab below is that original browser, moved under a tab and
	otherwise untouched; Library and Account are built on `dew.Library` and
	`dew.Account` (ADR-017/ADR-018). Account's own sign-in form (milestone
	26 sprint 3) is what finally lets this window replace `manage.rs`'s
	Win32 one, rather than only reading state that window still had to
	create.

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

	-- NEITHER HANDLER REBUILDS THE ROW LIST ITSELF (milestone 26 sprint 2;
	-- both used to call `rebuildLibraryRows()` here directly). `library.OnChange`
	-- below does that now, for every change regardless of source -- these
	-- two clicks are just one more source of one, the same as a terminal
	-- running `dew install` is. Only the STATUS MESSAGE is this handler's
	-- own to set, since `OnChange` has no way to know what a caller's own
	-- action was trying to do.
	toggleButton.Activated:Connect(function()
		local result = library.SetEnabled(entry.id, not entry.running)
		libraryStatus.Text = if result.ok
			then `{if entry.running then "Disabled" else "Enabled"} '{entry.id}'`
			else result.error
	end)

	uninstallButton.Activated:Connect(function()
		local result = library.Uninstall(entry.id)
		libraryStatus.Text = if result.ok then `Uninstalled '{entry.id}'` else result.error
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

-- THE ONE PLACE THE ROW LIST ACTUALLY GETS REBUILT, other than an explicit
-- Refresh click or the initial load below. Fires for a change from ANY
-- source: this dashboard's own Launch/Toggle/Uninstall, a crash, a plain
-- tray unload, or a separate `dew install`/`dew uninstall` run from a
-- terminal against this same running service (milestone 26) -- the point
-- of building it this way rather than re-polling on a timer.
--
-- NEVER TOUCHES `libraryStatus.Text` EXCEPT WHEN THE LIST IS EMPTY.
-- Clobbering it on every change would erase the result message a button's
-- own handler just set, one frame later -- the exact bug fixed earlier this
-- milestone for the manual-refresh case, reachable again here if this
-- called `refreshLibrary()` instead of `rebuildLibraryRows()` directly.
local function onLibraryChanged()
	local count = rebuildLibraryRows()
	if count == 0 then
		libraryStatus.Text = "No applets installed yet -- see the Store tab"
	end
end

library.OnChange(onLibraryChanged)

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
-- `onInstalled`; declaring it after `buildRow` would make that a capture of
-- a global instead of this local, since a Luau chunk resolves an upvalue at
-- the point a function is DEFINED, not where it is called.
local packages = source(nil :: { Package }?)

-- CALLBACKS, NOT AWAITING FLAGS (milestone 26; was a flag polled from
-- `desktop.Clock.OnFrame` through milestone 25 -- see `capabilities.rs` for
-- why that bookkeeping moved into the host). `Discover`/`Install` call these
-- back exactly once, themselves, the instant an answer lands; nothing here
-- polls for it any more.
local function onDiscovered(discovered)
	if discovered.ok then
		packages(discovered.packages)
		status.Text = if #discovered.packages == 0 then "No public packages yet" else ""
	else
		status.Text = discovered.error
	end
end

local function onInstalled(installed)
	status.Text = if installed.ok then `Installed '{installed.id}'` else installed.error
	-- THE LIBRARY TAB IS WHAT SHOWS AN INSTALL'S RESULT, since a newly
	-- installed applet is a Library entry, not a Store one -- refreshed
	-- here too so switching tabs right after an install shows it without a
	-- manual Refresh click.
	if installed.ok then
		refreshLibrary()
	end
end

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
		marketplace.Install(pkg.ownerUserId, pkg.appletId, onInstalled)
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

refreshButton.Activated:Connect(function()
	status.Text = "Refreshing..."
	marketplace.Discover(onDiscovered)
end)

status.Text = "Loading..."
marketplace.Discover(onDiscovered)

--------------------------------------------------------------------------
-- ACCOUNT: who is signed in, a way to sign out, and -- new this milestone
-- -- a way to sign IN, without a terminal. `dew.Account.SignIn` is
-- trigger-with-callback (the same shape Sprint 1 gave `Discover`/`Install`),
-- since `platform::login` is a real network call.
--
-- KNOWN LIMITATION, WORTH REVISITING: the password field below is a plain
-- `TextBox`, not masked. This DataModel's own `TextBox` has no password/
-- masking property the way `manage.rs`'s native `ES_PASSWORD` Win32 field
-- did -- carrying that behavior over means intercepting every keystroke and
-- substituting a placeholder character while tracking the real value
-- separately, which is real work this sprint did not do. Flagged rather
-- than silently shipped.
--------------------------------------------------------------------------

label(accountTab, "Title", UDim2.new(0, 16, 0, 12), UDim2.new(1, -32, 0, 24), 18, Color3.fromRGB(233, 239, 249)).Text =
	"Account"

-- SIGNED-IN STATE.
local accountStatus =
	label(accountTab, "Status", UDim2.new(0, 16, 0, 52), UDim2.new(1, -32, 0, 24), 13, Color3.fromRGB(206, 222, 246))
accountStatus.TextWrapped = true

local signOutButton = button(
	accountTab,
	"SignOut",
	UDim2.new(0, 16, 0, 84),
	UDim2.new(0, 96, 0, 28),
	"Sign out",
	Color3.fromRGB(90, 48, 48),
	Color3.fromRGB(240, 214, 214)
)

-- SIGNED-OUT STATE: a plain sign-in form. Built once, alongside the
-- signed-in controls above, and toggled by `refreshAccount` -- the same
-- "both sets exist, only one is visible" shape `manage.rs`'s own
-- `create_login_section` already used for the identical two states.
local emailLabel =
	label(accountTab, "EmailLabel", UDim2.new(0, 16, 0, 52), UDim2.new(0, 70, 0, 20), 12, Color3.fromRGB(146, 162, 188))
emailLabel.Text = "Email:"

local emailBox = Instance.new("TextBox")
emailBox.Name = "Email"
emailBox.Position = UDim2.new(0, 90, 0, 50)
emailBox.Size = UDim2.new(1, -106, 0, 24)
emailBox.BackgroundColor3 = Color3.fromRGB(28, 31, 40)
emailBox.TextColor3 = Color3.fromRGB(233, 239, 249)
emailBox.PlaceholderText = "you@example.com"
emailBox.PlaceholderColor3 = Color3.fromRGB(110, 122, 140)
emailBox.TextSize = 12
emailBox.ClearTextOnFocus = false
emailBox.Text = ""
emailBox.Parent = accountTab

local passwordLabel = label(
	accountTab,
	"PasswordLabel",
	UDim2.new(0, 16, 0, 84),
	UDim2.new(0, 70, 0, 20),
	12,
	Color3.fromRGB(146, 162, 188)
)
passwordLabel.Text = "Password:"

local passwordBox = Instance.new("TextBox")
passwordBox.Name = "Password"
passwordBox.Position = UDim2.new(0, 90, 0, 82)
passwordBox.Size = UDim2.new(1, -106, 0, 24)
passwordBox.BackgroundColor3 = Color3.fromRGB(28, 31, 40)
passwordBox.TextColor3 = Color3.fromRGB(233, 239, 249)
passwordBox.PlaceholderText = "Password"
passwordBox.PlaceholderColor3 = Color3.fromRGB(110, 122, 140)
passwordBox.TextSize = 12
passwordBox.ClearTextOnFocus = false
passwordBox.Text = ""
passwordBox.Parent = accountTab

local signInButton = button(
	accountTab,
	"SignIn",
	UDim2.new(0, 16, 0, 116),
	UDim2.new(0, 96, 0, 28),
	"Sign in",
	Color3.fromRGB(38, 52, 74),
	Color3.fromRGB(206, 222, 246)
)

local signInError =
	label(accountTab, "SignInError", UDim2.new(0, 16, 0, 152), UDim2.new(1, -32, 0, 40), 11, Color3.fromRGB(224, 148, 148))
signInError.Text = ""
signInError.TextWrapped = true
signInError.TextYAlignment = Enum.TextYAlignment.Top

local function setSignInFormVisible(visible: boolean)
	emailLabel.Visible = visible
	emailBox.Visible = visible
	passwordLabel.Visible = visible
	passwordBox.Visible = visible
	signInButton.Visible = visible
	signInError.Visible = visible
end

local function refreshAccount()
	local session = account.Whoami()
	if session then
		accountStatus.Text = `Signed in as {session.email}`
		accountStatus.Visible = true
		signOutButton.Visible = true
		setSignInFormVisible(false)
	else
		accountStatus.Visible = false
		signOutButton.Visible = false
		setSignInFormVisible(true)
	end
end

signOutButton.Activated:Connect(function()
	account.SignOut()
	refreshAccount()
end)

-- GUARDED THE SAME WAY `manage.rs`'s OWN `SIGNIN_IN_FLIGHT` GUARDED ITS
-- button: a second click before the first request returns is ignored
-- rather than starting a second sign-in, and the button's own label says
-- so while one is outstanding.
local signInInFlight = false

signInButton.Activated:Connect(function()
	if signInInFlight then
		return
	end
	local email = emailBox.Text
	local password = passwordBox.Text
	if email == "" or password == "" then
		signInError.Text = "Enter an email and password"
		return
	end

	signInInFlight = true
	signInButton.Text = "Signing in..."
	signInError.Text = ""

	account.SignIn(email, password, function(result)
		signInInFlight = false
		signInButton.Text = "Sign in"
		if result.ok then
			passwordBox.Text = ""
			refreshAccount()
		else
			signInError.Text = result.error
		end
	end)
end)

refreshAccount()

--------------------------------------------------------------------------
-- INITIAL STATE, for the tab that opens active by default.
--------------------------------------------------------------------------

refreshLibrary()

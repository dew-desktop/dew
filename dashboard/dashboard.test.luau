--!strict
--!tier math
--[=[
	The dashboard, driven through the real tree with real pointer clicks.

	WHAT THIS FILE COVERS:

	1. Navigation: each sidebar item shows its section, and the account card
	   at the sidebar's foot opens Account.
	2. Library against a FAKE `library.List()`: rows, the sidebar count, the
	   Running and Stopped filters, Refresh, Toggle calling `SetEnabled` with
	   the right `(id, enabled)` pair, and the Uninstall confirmation
	   `Aether.Dialog` (Cancel leaves the applet, Confirm uninstalls it and
	   the row goes). Dialog state is read through `ScreenStack.IsOpen`, not
	   `.Visible`, which lags the exit fade.
	3. Store against a fake `Discover`/`Install`: a package's row, Install
	   calling the fake with the right package, and Refresh discovering again.
	4. Account: the sign-in form while signed out, signing in and out, and
	   the account card following the session. A typed password paints only
	   bullets, and the sign-in handler still receives the real text.
	5. Fit: at four window widths, in every section and both account states,
	   every visible element lies inside its parent and every piece of text
	   that does not truncate fits its box.

	HOW THE FAKES GET IN: `dew test` gives this suite `desktop` and `DewTest`
	but no `dew` global, so this file sets one before requiring the
	dashboard. Modules share one global table here, so the dashboard's own
	top-level `assert(dew.Library, ...)` sees it.

	WHAT THIS FILE DOES NOT COVER:

	- The real `dew.Library`, `dew.Marketplace` and `dew.Account`. They read
	  real `installed.json` and coordinator state, or call the network, with
	  no seam for a fixture. This suite proves the dashboard's wiring only.
	- Keyboard and gamepad selection through the sidebar's `SelectionRouter`
	  wiring; only pointer clicks are driven.
	- Real keystrokes. `DewTest` has no keyboard, so typing is a write to the
	  focused TextBox's `Text`, which is what Dew's own key delivery does.

		cargo run -- test dashboard
]=]

local T = require("@testkit")
local ScreenStack = require("./roblox_packages/aether").ScreenStack

--------------------------------------------------------------------------
-- FAKES. In memory, built by this file alone.
--------------------------------------------------------------------------

type LibraryEntry = { id: string, name: string, description: string, enabled: boolean, running: boolean, bundled: boolean? }

-- TWO ENTRIES, so the Cancel scenario and the Confirm scenario each open a
-- dialog no other test has touched, and each dialog is told apart by the
-- name in its message.
local libraryEntries: { LibraryEntry } = {
	{ id = "sample-applet", name = "Sample Applet", description = "A fake entry for this test", enabled = true, running = false },
	{ id = "confirm-applet", name = "Confirm Applet", description = "Uninstalled via Confirm, once, in this suite", enabled = true, running = false },
}

local libraryOnChange: (() -> ())? = nil

local recorded = {
	setEnabled = {} :: { { id: string, enabled: boolean } },
	uninstall = {} :: { string },
	installed = {} :: { { owner: string, id: string } },
	discovered = 0,
	password = nil :: string?,
}

-- `SetEnabled` and `Uninstall` fire `OnChange` synchronously, as the real
-- ones do.
local fakeLibrary = {
	List = function(): { LibraryEntry }
		local copy = {}
		for i, entry in ipairs(libraryEntries) do
			copy[i] = entry
		end
		return copy
	end,
	OnChange = function(callback: () -> ())
		libraryOnChange = callback
	end,
	SetEnabled = function(id: string, enabled: boolean)
		table.insert(recorded.setEnabled, { id = id, enabled = enabled })
		for _, entry in ipairs(libraryEntries) do
			if entry.id == id then
				entry.running = enabled
			end
		end
		if libraryOnChange then
			libraryOnChange()
		end
		return { ok = true }
	end,
	Uninstall = function(id: string)
		table.insert(recorded.uninstall, id)
		for i, entry in ipairs(libraryEntries) do
			if entry.id == id then
				table.remove(libraryEntries, i)
				break
			end
		end
		if libraryOnChange then
			libraryOnChange()
		end
		return { ok = true }
	end,
}

local storePackages = {
	{ ownerUserId = "3f9c2a71-8d4e-4b0a-9e1f-6c2d7a5b8e03", appletId = "store-applet", uploadedAt = "2026-09-28" },
}

local fakeMarketplace = {
	Discover = function(callback: (any) -> ())
		recorded.discovered += 1
		callback({ ok = true, packages = storePackages })
	end,
	Install = function(owner: string, id: string, callback: (any) -> ())
		table.insert(recorded.installed, { owner = owner, id = id })
		callback({ ok = true, id = id })
	end,
}

local session: { email: string, userId: string }? = nil

local fakeAccount = {
	Whoami = function()
		return session
	end,
	SignOut = function()
		session = nil
	end,
	SignIn = function(email: string, password: string, callback: (any) -> ())
		recorded.password = password
		session = { email = email, userId = "9e4b6c20-7f31-4d8a-b5e6-2a1c0f93d7e4" }
		callback({ ok = true })
	end,
}

-- Set before `require("./dashboard")`: see this file's header.
dew = ({
	Marketplace = fakeMarketplace,
	Library = fakeLibrary,
	Account = fakeAccount,
} :: any)

require("./dashboard")

local root = (DewTest :: any).Root
local pointer = (DewTest :: any).Pointer
local harness = (DewTest :: any)

--- The first descendant named `name`, depth-first.
local function find(from: any, name: string): any
	for _, child in ipairs(from:GetChildren()) do
		if child.Name == name then
			return child
		end
		local deeper = find(child, name)
		if deeper then
			return deeper
		end
	end
	return nil
end

--- Every descendant named `name`, depth-first.
local function findAll(from: any, name: string, results: { any }?): { any }
	results = results or {}
	for _, child in ipairs(from:GetChildren()) do
		if child.Name == name then
			table.insert(results, child)
		end
		findAll(child, name, results)
	end
	return results :: { any }
end

local function click(x: number, y: number)
	pointer.Move(x, y)
	pointer.Down(x, y)
	pointer.Up(x, y)
end

local function clickCenter(element: any)
	local position, size = element.AbsolutePosition, element.AbsoluteSize
	click(position.X + size.X / 2, position.Y + size.Y / 2)
end

local HEIGHT = 560
local width = 720

local function settle()
	harness.Settle({ width = width, height = HEIGHT })
end

--- Settles, then clicks: layout has to have run before a click can land.
local function press(element: any)
	settle()
	clickCenter(element)
end

local function libraryTab(): any
	return find(root, "LibraryTab")
end

local function storeTab(): any
	return find(root, "StoreTab")
end

local function accountTab(): any
	return find(root, "AccountTab")
end

local function sidebar(): any
	return find(root, "Sidebar")
end

--------------------------------------------------------------------------
-- NAVIGATION
--------------------------------------------------------------------------

local function testLibraryTabIsActiveInitially()
	settle()
	assert(libraryTab().Visible, "Library is the dashboard's own initial section")
	assert(not storeTab().Visible, "Store should not be visible while Library is active")
	assert(not accountTab().Visible, "Account should not be visible while Library is active")
end

local function testClickingStoreTabSwitchesVisibility()
	press(find(root, "TabStore"))
	assert(storeTab().Visible, "clicking the Store tab should show StoreTab")
	assert(not libraryTab().Visible, "switching to Store should hide LibraryTab")
end

local function testClickingAccountTabSwitchesVisibility()
	press(find(root, "TabAccount"))
	assert(accountTab().Visible, "clicking the Account tab should show AccountTab")
	assert(not storeTab().Visible, "switching to Account should hide StoreTab")
end

local function testClickingLibraryTabReturnsToLibrary()
	press(find(root, "TabLibrary"))
	assert(libraryTab().Visible, "clicking the Library tab should show LibraryTab again")
	assert(not accountTab().Visible, "switching back to Library should hide AccountTab")
end

local function testTheAccountCardOpensAccount()
	press(find(root, "TabLibrary"))
	press(find(sidebar(), "Who"))
	assert(accountTab().Visible, "pressing the account card should show AccountTab")
	assert(not libraryTab().Visible, "pressing the account card should hide LibraryTab")
	press(find(root, "TabLibrary"))
end

local function testSidebarCountsFollowTheData()
	settle()
	local libraryCount = find(find(root, "TabLibrary"), "Count")
	local storeCount = find(find(root, "TabStore"), "Count")
	assert(libraryCount.Visible and libraryCount.Text == tostring(#libraryEntries), "Library's count is the installed total")
	assert(storeCount.Visible and storeCount.Text == tostring(#storePackages), "Store's count is what Discover returned")
	assert(find(sidebar(), "Primary").Text == "Not signed in", "the account card says nobody is signed in")
end

--------------------------------------------------------------------------
-- LIBRARY
--------------------------------------------------------------------------

--- The Library row whose `Name` label reads `entryName`.
local function findRow(entryName: string): any
	local list = find(libraryTab(), "List")
	if not list then
		return nil
	end
	for _, child in ipairs(list:GetChildren()) do
		if child.Name == "Row" then
			local nameLabel = find(child, "Name")
			if nameLabel and nameLabel.Text == entryName then
				return child
			end
		end
	end
	return nil
end

local function rowCount(listOwner: any): number
	local n = 0
	for _, child in ipairs(find(listOwner, "List"):GetChildren()) do
		if child.Name == "Row" then
			n += 1
		end
	end
	return n
end

--- The Uninstall confirmation dialog whose message names `entryName`.
local function findDialog(entryName: string): any
	for _, dialog in ipairs(findAll(root, "UninstallConfirm")) do
		local message = find(dialog, "Message")
		if message and string.find(message.Text, entryName, 1, true) then
			return dialog
		end
	end
	return nil
end

local function testLibraryRowRendersFakeListData()
	press(find(root, "TabLibrary"))
	local row = findRow("Sample Applet")
	assert(row ~= nil, "a row should be drawn for the fake's 'Sample Applet' entry")
	assert(
		find(row, "Description").Text == "A fake entry for this test",
		"the row's own Description label should read the fake entry's description"
	)
	assert(find(row, "Detail").Text == "sample-applet", "the row's Detail line should read the entry's id")
end

local function testToggleCallsFakeSetEnabledWithTheRightArguments()
	press(find(root, "TabLibrary"))
	local before = #recorded.setEnabled
	press(find(findRow("Sample Applet"), "Toggle"))
	assert(#recorded.setEnabled == before + 1, "clicking Toggle should call library.SetEnabled exactly once")
	local call = recorded.setEnabled[#recorded.setEnabled]
	assert(call.id == "sample-applet", "SetEnabled should be called with this row's own entry id")
	assert(call.enabled == true, "the entry started not-running, so Toggle should ask to enable it")
	assert(find(findRow("Sample Applet"), "Toggle").Label.Text == "Disable", "a running applet's Toggle reads Disable")
end

local function testARebuildLeavesOneDialogPerRow()
	-- The Toggle above rebuilt every row. Each row owns one dialog, and the
	-- rebuild destroys the old ones.
	local dialogs = #findAll(root, "UninstallConfirm")
	assert(dialogs == rowCount(libraryTab()), `one dialog per row after a rebuild, found {dialogs}`)
end

local function testTheFiltersShowRunningAndStoppedRows()
	press(find(root, "TabLibrary"))
	press(find(libraryTab(), "FilterRunning"))
	assert(findRow("Sample Applet") ~= nil, "Running shows the running applet")
	assert(findRow("Confirm Applet") == nil, "Running hides the stopped applet")
	assert(find(libraryTab(), "FilterRunning").Text == "Running 1", "the Running chip counts running applets")

	press(find(libraryTab(), "FilterStopped"))
	assert(findRow("Sample Applet") == nil, "Stopped hides the running applet")
	assert(findRow("Confirm Applet") ~= nil, "Stopped shows the stopped applet")

	press(find(libraryTab(), "FilterAll"))
	assert(findRow("Sample Applet") ~= nil and findRow("Confirm Applet") ~= nil, "All shows both")
end

local function testUninstallOpensDialogAndCancelDoesNotUninstall()
	press(find(root, "TabLibrary"))
	local before = #recorded.uninstall
	press(find(findRow("Sample Applet"), "Uninstall"))
	assert(ScreenStack.IsOpen("uninstall-sample-applet")(), "clicking Uninstall should open the confirmation dialog")
	assert(#recorded.uninstall == before, "opening the dialog must not call library.Uninstall by itself")

	-- `press` settles again first: the dialog's buttons only have geometry
	-- once it is open.
	press(find(findDialog("Sample Applet"), "Cancel"))
	assert(
		not ScreenStack.IsOpen("uninstall-sample-applet")(),
		"Cancel should close the confirmation dialog"
	)
	assert(#recorded.uninstall == before, "Cancel must never call library.Uninstall")
	assert(findRow("Sample Applet") ~= nil, "the applet should still be in the fake's own list after Cancel")
end

local function testUninstallConfirmCallsFakeUninstallAndRemovesTheRow()
	press(find(root, "TabLibrary"))
	press(find(findRow("Confirm Applet"), "Uninstall"))
	assert(ScreenStack.IsOpen("uninstall-confirm-applet")(), "Uninstall should open the confirmation dialog")

	press(find(findDialog("Confirm Applet"), "Confirm"))
	assert(#recorded.uninstall == 1, "Confirm should call library.Uninstall exactly once")
	assert(recorded.uninstall[1] == "confirm-applet", "Confirm should call library.Uninstall with this row's own entry id")
	assert(findRow("Confirm Applet") == nil, "the fake's own OnChange should have rebuilt the list without the uninstalled row")
	assert(findRow("Sample Applet") ~= nil, "uninstalling one entry should not touch the other")
	assert(findDialog("Confirm Applet") == nil, "the confirmed dialog is gone")
end

local function testLibraryRefreshRebuildsFromList()
	press(find(root, "TabLibrary"))
	-- Added behind the dashboard's back: no OnChange fires.
	table.insert(libraryEntries, { id = "late-applet", name = "Late Applet", description = "", enabled = true, running = false })
	assert(findRow("Late Applet") == nil, "nothing has told the dashboard yet")
	press(find(find(libraryTab(), "Header"), "Refresh"))
	local row = findRow("Late Applet")
	assert(row ~= nil, "Refresh should read List again and draw the new entry")
	assert(find(row, "Description").Text == "late-applet", "an entry with no description shows its id")
	assert(find(find(root, "TabLibrary"), "Count").Text == tostring(#libraryEntries), "Refresh updates the count")
end

local function testABundledRowHasToggleButNoUninstall()
	press(find(root, "TabLibrary"))
	table.insert(libraryEntries, {
		id = "bundled-applet",
		name = "Bundled Applet",
		description = "Ships with Dew",
		enabled = false,
		running = false,
		bundled = true,
	})
	if libraryOnChange then
		libraryOnChange()
	end
	local row = findRow("Bundled Applet")
	assert(row ~= nil, "a bundled entry is listed like any other")
	assert(find(row, "Toggle") ~= nil, "a bundled entry can be enabled and disabled")
	assert(find(row, "Uninstall") == nil, "a bundled entry has no Uninstall button")
	assert(findDialog("Bundled Applet") == nil, "a bundled entry builds no Uninstall dialog")

	table.remove(libraryEntries, #libraryEntries)
	if libraryOnChange then
		libraryOnChange()
	end
	assert(findRow("Bundled Applet") == nil, "the entry is gone again for the tests that follow")
end

--------------------------------------------------------------------------
-- STORE
--------------------------------------------------------------------------

local function testStoreRowInstallsThroughTheFake()
	press(find(root, "TabStore"))
	local row = find(find(storeTab(), "List"), "Row")
	assert(row ~= nil, "Discover's package should have a row")
	assert(find(row, "Name").Text == "store-applet", "the row is named by its applet id")
	assert(
		find(row, "Detail").Text == "@3f9c2a71-8d4e-4b0a-9e1f-6c2d7a5b8e03/store-applet",
		"the Detail line is the install handle"
	)

	press(find(row, "Install"))
	local call = recorded.installed[#recorded.installed]
	assert(call ~= nil, "Install should call marketplace.Install")
	assert(call.owner == storePackages[1].ownerUserId and call.id == "store-applet", "Install passes this row's package")
	assert(find(storeTab(), "Status").Text == "Installed 'store-applet'", "the result is reported")
end

local function testStoreRefreshDiscoversAgain()
	press(find(root, "TabStore"))
	local before = recorded.discovered
	press(find(find(storeTab(), "Header"), "Refresh"))
	assert(recorded.discovered == before + 1, "Refresh should call Discover again")
	assert(rowCount(storeTab()) == #storePackages, "Refresh redraws one row per package")
end

--------------------------------------------------------------------------
-- ACCOUNT
--------------------------------------------------------------------------

local function testSignedOutShowsTheSignInForm()
	press(find(root, "TabAccount"))
	assert(find(accountTab(), "Email").Visible, "signed out should show the email field")
	assert(find(accountTab(), "Password").Visible, "signed out should show the password field")
	assert(find(accountTab(), "SignIn").Visible, "signed out should show the Sign in button")
	assert(not find(accountTab(), "SignOut").Visible, "signed out should hide Sign out")
end

--- The TextBox under `from`, itself included, that holds keyboard focus.
local function focusedTextBox(from: any): any
	if from:IsA("TextBox") and from:IsFocused() then
		return from
	end
	for _, child in ipairs(from:GetChildren()) do
		local found = focusedTextBox(child)
		if found then
			return found
		end
	end
	return nil
end

--- Clicks `field`, then types `text` into whichever TextBox that focused.
local function typeInto(field: any, text: string)
	press(field)
	local box = focusedTextBox(root)
	assert(box ~= nil, `clicking {field.Name} should focus a TextBox`)
	box.Text = text
	settle()
end

local function signIn(email: string, password: string?)
	press(find(root, "TabAccount"))
	typeInto(find(accountTab(), "Email"), email)
	typeInto(find(accountTab(), "Password"), password or "hunter2")
	press(find(accountTab(), "SignIn"))
end

--- Is `node` drawn: visible itself and under nothing hidden.
local function drawn(node: any): boolean
	while node and node ~= root do
		if node:IsA("GuiObject") and not node.Visible then
			return false
		end
		node = node.Parent
	end
	return true
end

--- Every drawn text object under `from` whose Text contains `needle`.
local function drawnTextsHolding(from: any, needle: string, results: { string }?): { string }
	results = results or {}
	for _, child in ipairs(from:GetChildren()) do
		local isText = child:IsA("TextBox") or child:IsA("TextLabel") or child:IsA("TextButton")
		if isText and drawn(child) and string.find(child.Text, needle, 1, true) then
			table.insert(results :: { string }, `{child.Parent.Name}.{child.Name} ({child.ClassName})`)
		end
		drawnTextsHolding(child, needle, results)
	end
	return results :: { string }
end

local BULLET = "\u{2022}"

local function testATypedPasswordPaintsOnlyBullets()
	press(find(root, "TabAccount"))
	local field = find(accountTab(), "Password")
	typeInto(field, "hunter2")
	local label = find(field, "DisplayLabel")
	assert(label ~= nil, "the password field should paint its text through a DisplayLabel")
	assert(label.Text == string.rep(BULLET, 7), `seven typed characters should paint seven bullets, got "{label.Text}"`)
	local leaks = drawnTextsHolding(root, "hunter2")
	assert(#leaks == 0, `no drawn text may hold the typed password, found: {table.concat(leaks, ", ")}`)
	press(find(root, "TabLibrary"))
end

local function testSignInReceivesTheRealPasswordAndClearsTheField()
	signIn("person@example.com", "correct horse")
	assert(recorded.password == "correct horse", `SignIn should receive the typed text, got "{tostring(recorded.password)}"`)
	press(find(accountTab(), "SignOut"))
	local label = find(find(accountTab(), "Password"), "DisplayLabel")
	assert(label ~= nil and label.Text == "Password", "after signing in, the password field is empty again and shows its placeholder")
	press(find(root, "TabLibrary"))
end

local function testSigningInSwitchesToSignedInChrome()
	signIn("person@example.com")

	assert(not find(accountTab(), "Email").Visible, "signing in should hide the email field")
	assert(not find(accountTab(), "Password").Visible, "signing in should hide the password field")
	assert(find(accountTab(), "SignOut").Visible, "signing in should show Sign out")
	assert(
		find(accountTab(), "Status").Text == "Signed in as person@example.com",
		"the status label should read the stubbed SignIn's own session email"
	)
	assert(find(sidebar(), "Primary").Text == "person@example.com", "the account card shows who is signed in")
end

local function testSigningOutReturnsToTheSignInForm()
	press(find(root, "TabAccount"))
	press(find(accountTab(), "SignOut"))
	assert(session == nil, "Sign out should call account.SignOut")
	assert(find(accountTab(), "Email").Visible, "signing out should show the email field again")
	assert(not find(accountTab(), "SignOut").Visible, "signing out should hide Sign out")
	assert(find(sidebar(), "Primary").Text == "Not signed in", "the account card says nobody is signed in")
end

--------------------------------------------------------------------------
-- FIT
--------------------------------------------------------------------------

local TOLERANCE = 1

--- `Parent.Child.Name` up to the root, for a readable failure.
local function path(node: any): string
	local names = {}
	while node and node ~= root do
		table.insert(names, 1, node.Name)
		node = node.Parent
	end
	return table.concat(names, ".")
end

--- Every visible GuiObject under `node` that pokes outside its parent, and
--- every visible text that neither truncates nor fits its box.
local function misfits(node: any, problems: { string })
	local parentIsGui = node:IsA("GuiObject")
	local scrolls = node:IsA("ScrollingFrame")
	for _, child in ipairs(node:GetChildren()) do
		if child:IsA("GuiObject") and child.Visible then
			local p, s = child.AbsolutePosition, child.AbsoluteSize
			if parentIsGui then
				local pp, ps = node.AbsolutePosition, node.AbsoluteSize
				local outsideX = p.X < pp.X - TOLERANCE or p.X + s.X > pp.X + ps.X + TOLERANCE
				-- A scrolling list's rows run below its visible box on purpose.
				local outsideY = not scrolls and (p.Y < pp.Y - TOLERANCE or p.Y + s.Y > pp.Y + ps.Y + TOLERANCE)
				if outsideX or outsideY then
					table.insert(problems, `{path(child)} at {p} size {s} outside {node.Name} at {pp} size {ps}`)
				end
			end
			local isText = child:IsA("TextLabel") or child:IsA("TextButton")
			if isText and child.Text ~= "" and child.TextTruncate == Enum.TextTruncate.None and not child.TextFits then
				table.insert(problems, `{path(child)} does not fit "{child.Text}" ({child.TextBounds} in {s})`)
			end
			misfits(child, problems)
		end
	end
end

local function assertFits(what: string)
	settle()
	local problems = {}
	misfits(root, problems)
	assert(#problems == 0, `{what} at {width}x{HEIGHT}:\n  {table.concat(problems, "\n  ")}`)
end

local function testNothingOverflowsAtAnyTestedWidth()
	for _, w in { 420, 520, 720, 960 } do
		width = w
		press(find(root, "TabLibrary"))
		assertFits("Library")
		press(find(root, "TabStore"))
		assertFits("Store")
		press(find(root, "TabAccount"))
		assertFits("Account, signed out")
		signIn("riley.moss@example.com")
		assertFits("Account, signed in")
		press(find(accountTab(), "SignOut"))
	end
	width = 720
end

T.test("Library is the active tab initially", testLibraryTabIsActiveInitially, "math")
T.test("clicking the Store tab switches visibility", testClickingStoreTabSwitchesVisibility, "math")
T.test("clicking the Account tab switches visibility", testClickingAccountTabSwitchesVisibility, "math")
T.test("clicking the Library tab returns to Library", testClickingLibraryTabReturnsToLibrary, "math")
T.test("the account card opens Account", testTheAccountCardOpensAccount, "math")
T.test("the sidebar counts follow the data", testSidebarCountsFollowTheData, "math")
T.test("a Library row renders the fake's own List data", testLibraryRowRendersFakeListData, "math")
T.test("Toggle calls the fake's own SetEnabled with the right arguments", testToggleCallsFakeSetEnabledWithTheRightArguments, "math")
T.test("a rebuild leaves one Uninstall dialog per row", testARebuildLeavesOneDialogPerRow, "math")
T.test("the filters show running and stopped rows", testTheFiltersShowRunningAndStoppedRows, "math")
T.test("Uninstall opens the dialog and Cancel does not uninstall", testUninstallOpensDialogAndCancelDoesNotUninstall, "math")
T.test("Uninstall's Confirm calls the fake's Uninstall and removes the row", testUninstallConfirmCallsFakeUninstallAndRemovesTheRow, "math")
T.test("Library's Refresh rebuilds the rows from List", testLibraryRefreshRebuildsFromList, "math")
T.test("a bundled row can be toggled but not uninstalled", testABundledRowHasToggleButNoUninstall, "math")
T.test("a Store row installs through the fake", testStoreRowInstallsThroughTheFake, "math")
T.test("Store's Refresh discovers again", testStoreRefreshDiscoversAgain, "math")
T.test("signed out shows the sign-in form", testSignedOutShowsTheSignInForm, "math")
T.test("signing in switches to signed-in chrome", testSigningInSwitchesToSignedInChrome, "math")
T.test("signing out returns to the sign-in form", testSigningOutReturnsToTheSignInForm, "math")
T.test("a typed password paints only bullets", testATypedPasswordPaintsOnlyBullets, "math")
T.test("sign in receives the real password and clears the field", testSignInReceivesTheRealPasswordAndClearsTheField, "math")
T.test("nothing overflows at any tested width", testNothingOverflowsAtAnyTestedWidth, "math")

return T.run("dashboard")

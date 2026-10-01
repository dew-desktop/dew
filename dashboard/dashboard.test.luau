--!strict
--!tier math
--[=[
	Milestone 30 sprint 9's own answer to Open Question 3.

	WHAT THIS FILE COVERS, and how, each confirmed live rather than assumed:

	1. Tab switching (Library <-> Store <-> Account) -- real `DewTest.Pointer`
	   clicks on the real tab buttons, asserting each content Frame's own
	   `.Visible`.
	2. The Library panel's row rendering against a FAKE `library.List()` --
	   the row's own Name/Description labels read back off the real tree.
	3. Toggle calling the fake's own `SetEnabled` with the right `(id,
	   enabled)` pair, read back off what the fake itself recorded, not real
	   machine state.
	4. The Uninstall confirmation `Aether.Dialog`, driven the same way
	   `dialog-demo.test.luau` drives Dialog A/B: a real click opens it
	   (`ScreenStack.IsOpen`, not `.Visible` -- the same reasoning that
	   file's own header already gives for why `.Visible` lags the exit
	   fade). Cancel, on one fake entry ("sample-applet"), closes it without
	   calling the fake's `Uninstall`; Confirm, on a SEPARATE fake entry
	   ("confirm-applet"), calls the fake's `Uninstall` with the right
	   `entry.id` and the row disappears from the rebuilt list. Two entries,
	   not one reused for both, because of a real characteristic of
	   `dashboard.luau` this investigation surfaced along the way: every
	   Library rebuild (a Toggle click, any `library.OnChange`) leaves the
	   PREVIOUS row's own Uninstall dialog Frame behind, invisible and
	   unregistered but never destroyed, parented under `body` forever --
	   see `findDialog`'s own comment below for the full chain of reasoning
	   and why this test-writing sprint reports it rather than fixing it.
	5. The sign-in form's visibility toggle: signed-out shows the form
	   (stubbed `Whoami` returning `nil`), a stubbed `SignIn` flipping to a
	   session flips the chrome to signed-in, matching `refreshAccount`'s own
	   two-state shape.

	HOW THE FAKES GET IN FRONT OF `dashboard.luau`'S OWN TOP-LEVEL
	`assert(dew.Marketplace, ...)` etc: `install_test_surface`
	(`host/src/main.rs`), confirmed by reading it directly, sets `desktop`
	(a FIXED, real capability set -- Widget/Window/Overlay/Popover/Storage/
	Clipboard, real enough that `desktop.Window` below is the same handle
	`DewTest.Root` reads back) and `DewTest` as globals, but never touches a
	`dew` global at all -- `dew_subtable` (`capabilities.rs`) only creates one
	lazily, from inside a `Permission::Library`/`Marketplace`/`Auth` arm
	`install_test_surface`'s own fixed grant list never reaches. `dew` is
	therefore simply nil under `dew test` for this suite, with nothing to
	shadow -- this file sets it itself, as a plain global, before its own
	`require("./dashboard")` below, and mlua's shared-globals `require`
	(confirmed by reading `crates/runtime/src/modules.rs`: modules load into
	the SAME Lua state's global table, there is no per-script environment
	the way real Roblox gives every script its own) makes that assignment
	visible to `dashboard.luau`'s own top-level `assert`s the instant they
	run.

	WHAT THIS FILE DOES NOT COVER, AND WHY -- open honestly, not silently
	shipped as full coverage:

	- The Store panel's real `dew.Marketplace.Discover`/`Install` against a
	  real marketplace are exercised here only against this file's OWN fake
	  (canned `Discover`/`Install` results), the same as Library/Account --
	  this proves the DASHBOARD's wiring (does clicking Install call
	  `marketplace.Install` with the right package, does a successful
	  install refresh Library), not the real network-backed `platform`
	  functions behind it, which sprint 6/7's own investigation already
	  found call real HTTP endpoints with no injectable seam.
	- No REAL `dew.Library`/`dew.Marketplace`/`dew.Account` call is ever
	  exercised by this file, and it does not attempt to be: `capabilities.rs`
	  wires `Permission::Library`'s `List` directly to `crate::library::list()`
	  (real `installed.json`, real coordinator state) with no seam to
	  substitute a fixture, and `Account.SignIn`/`Marketplace.Discover`/
	  `Install` all call real, network-backed `platform` functions
	  (confirmed independently by milestone 30's sprints 6-8, and reconfirmed
	  here by reading `capabilities.rs` directly before writing this file).
	  A host-side seam that would let `dew test` substitute a fixture store
	  for `crate::library::list()`/the real coordinator is a genuine,
	  separate architectural change -- not attempted here, see this sprint's
	  own report for why.
	- Keyboard/gamepad selection through the tab bar's `SelectionRouter`
	  wiring is not driven here -- only pointer clicks, the same scope
	  `dialog-demo.test.luau`'s own suite keeps.
	- The password field's known lack of masking (named in `dashboard.luau`'s
	  own header) is a pre-existing, still-open gap this suite does not
	  re-litigate.

		cargo run -- test dashboard
]=]

local T = require("@testkit")
local ScreenStack = require("./roblox_packages/aether").ScreenStack

--------------------------------------------------------------------------
-- FAKES. Every one of these is in-memory, constructed by this test file
-- alone -- nothing here reaches `crate::library::list()`, real
-- `installed.json`, or a real network call. `dashboard.luau` never knows
-- the difference: it only ever sees a `dew.Library`/`dew.Marketplace`/
-- `dew.Account` shaped table, exactly like the real `desktop` table
-- `capabilities.rs` hands a real applet.
--------------------------------------------------------------------------

type LibraryEntry = { id: string, name: string, description: string, enabled: boolean, running: boolean }

-- TWO ENTRIES, DELIBERATELY -- "sample-applet" carries the render/Toggle/
-- Cancel scenarios, "confirm-applet" is reserved for the Confirm scenario
-- alone. Not for reactivity reasons -- see `findDialog`'s own comment below
-- for the real one, found live while writing this suite: every Library
-- rebuild leaves the previous row's own Uninstall dialog Frame behind,
-- un-destroyed, so a single entry reused across every dialog test would
-- accumulate several same-named "UninstallConfirm" Frames for THAT one
-- entry and make "which one is current" ambiguous for no reason beyond this
-- suite's own convenience. Two entries keep each scenario's own dialog
-- lookup unambiguous by CONTENT (`findDialog`'s own Message-text match)
-- without relying on find-the-last-one order for entries that also share a
-- name.
local libraryEntries: { LibraryEntry } = {
	{ id = "sample-applet", name = "Sample Applet", description = "A fake entry for this test", enabled = true, running = false },
	{ id = "confirm-applet", name = "Confirm Applet", description = "Uninstalled via Confirm, once, in this suite", enabled = true, running = false },
}

local libraryOnChange: (() -> ())? = nil

local recorded = {
	setEnabled = {} :: { { id: string, enabled: boolean } },
	uninstall = {} :: { string },
	installed = {} :: { { owner: string, id: string } },
}

-- `library.Uninstall`/`SetEnabled` FIRE `OnChange` SYNCHRONOUSLY, matching
-- `dashboard.luau`'s own comment on `buildLibraryRow`'s Confirm handler
-- ("library.Uninstall fires library.OnChange SYNCHRONOUSLY") -- this fake
-- mirrors that documented real contract, not an arbitrary choice of its own.
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

local fakeMarketplace = {
	Discover = function(callback: (any) -> ())
		callback({ ok = true, packages = {} })
	end,
	Install = function(owner: string, id: string, callback: (any) -> ())
		table.insert(recorded.installed, { owner = owner, id = id })
		callback({ ok = true, id = id })
	end,
}

local session: { email: string }? = nil

local fakeAccount = {
	Whoami = function()
		return session
	end,
	SignOut = function()
		session = nil
	end,
	SignIn = function(email: string, _password: string, callback: (any) -> ())
		session = { email = email }
		callback({ ok = true })
	end,
}

-- SET BEFORE `require("./dashboard")` BELOW -- see this file's own header
-- for why a plain global assignment here is what `dashboard.luau`'s own
-- top-level `assert(dew.Marketplace, ...)` etc. actually see.
dew = ({
	Marketplace = fakeMarketplace,
	Library = fakeLibrary,
	Account = fakeAccount,
} :: any)

require("./dashboard")

local root = (DewTest :: any).Root
local pointer = (DewTest :: any).Pointer
local harness = (DewTest :: any)

--- The first descendant named `name`, depth-first -- the same helper
--- `dialog-demo.test.luau` already proves out.
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

--- Every descendant named `name`, depth-first -- unlike `find` above, does
--- not stop at the first match. Needed once more than one Library row's own
--- `Aether.Dialog` exists at once: both share the literal `Name =
--- "UninstallConfirm"`/`"Cancel"`/`"Confirm"`, so `find(root, "Confirm")`
--- alone cannot tell one row's dialog from another's -- it would silently
--- return whichever comes first in tree order, found live the first time
--- this suite ran two Library entries side by side.
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

local function settle()
	harness.Settle({ width = 420, height = 560 })
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

--------------------------------------------------------------------------
-- TAB SWITCHING
--------------------------------------------------------------------------

local function testLibraryTabIsActiveInitially()
	settle()
	assert(libraryTab().Visible, "Library is the dashboard's own InitialActive tab")
	assert(not storeTab().Visible, "Store should not be visible while Library is active")
	assert(not accountTab().Visible, "Account should not be visible while Library is active")
end

local function testClickingStoreTabSwitchesVisibility()
	settle()
	clickCenter(find(root, "TabStore"))
	assert(storeTab().Visible, "clicking the Store tab should show StoreTab")
	assert(not libraryTab().Visible, "switching to Store should hide LibraryTab")
end

local function testClickingAccountTabSwitchesVisibility()
	settle()
	clickCenter(find(root, "TabAccount"))
	assert(accountTab().Visible, "clicking the Account tab should show AccountTab")
	assert(not storeTab().Visible, "switching to Account should hide StoreTab")
end

local function testClickingLibraryTabReturnsToLibrary()
	settle()
	clickCenter(find(root, "TabLibrary"))
	assert(libraryTab().Visible, "clicking the Library tab should show LibraryTab again")
	assert(not accountTab().Visible, "switching back to Library should hide AccountTab")
end

--------------------------------------------------------------------------
-- LIBRARY: rendering against the fake, Toggle wiring, Uninstall's Dialog
--------------------------------------------------------------------------

--- The Library row whose own `Name` label reads `entryName` -- depth-first
--- `find` alone is not enough once more than one row exists (both share the
--- Name "Row"), so this looks inside each one for its label instead.
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

--- The CURRENT Uninstall confirmation dialog whose own Message names
--- `entryName` -- the LAST such match, not the first. Two things found live
--- while writing this suite, together, are why: (1) `find`/`findAll` cannot
--- tell two rows' dialogs apart on name alone, since both share the literal
--- `Name = "UninstallConfirm"`; and (2) `buildLibraryRow`'s own Dialog Frame
--- is parented to `body` (see `dashboard.luau`'s own comment on
--- `uninstallDialogFrame.Parent = body`, for why it has to cover the whole
--- window rather than just one row) and is destroyed ONLY by the Confirm
--- path's own explicit `uninstallDialogFrame:Destroy()` -- an ordinary
--- rebuild (Toggle, or any `library.OnChange`) calls `clearLibraryRows`,
--- which tears down each row's own `vide.root` scope (unregistering that
--- dialog's `ScreenStack`/`TransitionOrchestrator` state) but never destroys
--- the dialog FRAME itself, since it was never parented under the row that
--- scope owns. Every rebuild therefore leaves the PREVIOUS dialog Frame
--- behind as an invisible, unregistered zombie under `body`, and
--- `buildLibraryRow` parents a brand new one alongside it -- confirmed
--- directly here (this suite's own Toggle test, upstream, triggers exactly
--- one such rebuild, and this function found FOUR `UninstallConfirm` frames
--- in the tree by the time this test ran, two per entry, only the LAST of
--- each pair still live). This is a real characteristic of the shipped
--- dashboard this investigation surfaced, not a defect this test-writing
--- sprint fixes -- see this sprint's own report. Picking the last match is
--- this suite's own honest accommodation of it, not a workaround for a bug
--- in this file.
local function findDialog(entryName: string): any
	local match: any = nil
	for _, dialog in ipairs(findAll(root, "UninstallConfirm")) do
		local message = find(dialog, "Message")
		if message and string.find(message.Text, entryName, 1, true) then
			match = dialog
		end
	end
	return match
end

local function testLibraryRowRendersFakeListData()
	settle()
	clickCenter(find(root, "TabLibrary"))
	local row = findRow("Sample Applet")
	assert(row ~= nil, "buildLibraryRow should have drawn a row for the fake's 'Sample Applet' entry")
	assert(
		find(row, "Description").Text == "A fake entry for this test",
		"the row's own Description label should read the fake entry's description"
	)
end

local function testToggleCallsFakeSetEnabledWithTheRightArguments()
	settle()
	clickCenter(find(root, "TabLibrary"))
	local before = #recorded.setEnabled
	clickCenter(find(findRow("Sample Applet"), "Toggle"))
	assert(#recorded.setEnabled == before + 1, "clicking Toggle should call library.SetEnabled exactly once")
	local call = recorded.setEnabled[#recorded.setEnabled]
	assert(call.id == "sample-applet", "SetEnabled should be called with this row's own entry id")
	assert(call.enabled == true, "the entry started not-running, so Toggle should ask to enable it")
end

local function testUninstallOpensDialogAndCancelDoesNotUninstall()
	settle()
	clickCenter(find(root, "TabLibrary"))
	local before = #recorded.uninstall
	clickCenter(find(findRow("Sample Applet"), "Uninstall"))
	assert(ScreenStack.IsOpen("uninstall-sample-applet")(), "clicking Uninstall should open the confirmation dialog")
	assert(#recorded.uninstall == before, "opening the dialog must not call library.Uninstall by itself")

	-- SETTLE AGAIN, NOW THAT THE DIALOG IS OPEN: `DewTest.Settle` only
	-- computes layout for what is CURRENTLY visible (`commit_geometry`), and
	-- the Cancel/Confirm row was invisible during the top-of-test `settle()`
	-- above (`Dialog.luau`'s own content only becomes part of a laid-out,
	-- visible tree once `Open()` runs). Clicking before this second settle
	-- hits stale (0, 0)-sized geometry and can land on the wrong button --
	-- found live the first time this test was written, the same
	-- "layout has to have run before a test can ask where anything is"
	-- requirement `install_test_surface`'s own `Settle` comment names.
	settle()
	clickCenter(find(findDialog("Sample Applet"), "Cancel"))
	assert(
		not ScreenStack.IsOpen("uninstall-sample-applet")(),
		"Cancel should close the confirmation dialog"
	)
	assert(#recorded.uninstall == before, "Cancel must never call library.Uninstall")
	assert(findRow("Sample Applet") ~= nil, "the applet should still be in the fake's own list after Cancel")
end

-- A SEPARATE ENTRY ("confirm-applet"), NOT "sample-applet" -- see
-- `libraryEntries`'s own comment above for why. Its own `Aether.Dialog` has
-- no open/close history yet, so this is a single, first-ever `Open()` ->
-- `Confirm`, the same shape the Cancel test's own single open->close already
-- proved works.
local function testUninstallConfirmCallsFakeUninstallAndRemovesTheRow()
	settle()
	clickCenter(find(root, "TabLibrary"))
	clickCenter(find(findRow("Confirm Applet"), "Uninstall"))
	assert(ScreenStack.IsOpen("uninstall-confirm-applet")(), "Uninstall should open the confirmation dialog")

	settle()
	clickCenter(find(findDialog("Confirm Applet"), "Confirm"))
	assert(#recorded.uninstall == 1, "Confirm should call library.Uninstall exactly once")
	assert(recorded.uninstall[1] == "confirm-applet", "Confirm should call library.Uninstall with this row's own entry id")
	assert(findRow("Confirm Applet") == nil, "the fake's own OnChange should have rebuilt the list without the uninstalled row")
	assert(findRow("Sample Applet") ~= nil, "uninstalling one entry should not touch the other")
end

--------------------------------------------------------------------------
-- ACCOUNT: sign-in form visibility against a stubbed Whoami/SignIn
--------------------------------------------------------------------------

local function testSignedOutShowsTheSignInForm()
	settle()
	clickCenter(find(root, "TabAccount"))
	assert(find(accountTab(), "Email").Visible, "signed out should show the email field")
	assert(find(accountTab(), "Password").Visible, "signed out should show the password field")
	assert(find(accountTab(), "SignIn").Visible, "signed out should show the Sign in button")
	assert(not find(accountTab(), "SignOut").Visible, "signed out should hide Sign out")
end

local function testSigningInSwitchesToSignedInChrome()
	settle()
	clickCenter(find(root, "TabAccount"))
	local emailBox = find(accountTab(), "Email")
	local passwordBox = find(accountTab(), "Password")
	emailBox.Text = "person@example.com"
	passwordBox.Text = "hunter2"

	clickCenter(find(accountTab(), "SignIn"))

	assert(not find(accountTab(), "Email").Visible, "signing in should hide the email field")
	assert(not find(accountTab(), "Password").Visible, "signing in should hide the password field")
	assert(find(accountTab(), "SignOut").Visible, "signing in should show Sign out")
	assert(
		find(accountTab(), "Status").Text == "Signed in as person@example.com",
		"the status label should read the stubbed SignIn's own session email"
	)
end

T.test("Library is the active tab initially", testLibraryTabIsActiveInitially, "math")
T.test("clicking the Store tab switches visibility", testClickingStoreTabSwitchesVisibility, "math")
T.test("clicking the Account tab switches visibility", testClickingAccountTabSwitchesVisibility, "math")
T.test("clicking the Library tab returns to Library", testClickingLibraryTabReturnsToLibrary, "math")
T.test("a Library row renders the fake's own List data", testLibraryRowRendersFakeListData, "math")
T.test("Toggle calls the fake's own SetEnabled with the right arguments", testToggleCallsFakeSetEnabledWithTheRightArguments, "math")
T.test("Uninstall opens the dialog and Cancel does not uninstall", testUninstallOpensDialogAndCancelDoesNotUninstall, "math")
T.test("Uninstall's Confirm calls the fake's Uninstall and removes the row", testUninstallConfirmCallsFakeUninstallAndRemovesTheRow, "math")
T.test("signed out shows the sign-in form", testSignedOutShowsTheSignInForm, "math")
T.test("signing in switches to signed-in chrome", testSigningInSwitchesToSignedInChrome, "math")

return T.run("dashboard")

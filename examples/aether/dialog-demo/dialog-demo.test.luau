--!strict
--!tier math
--[=[
	milestone 60 sprint B2's own live verification of Dialog.luau, through the
	real host (DewTest.Pointer, the same harness contextmenu.test.luau
	already proved out for a menu) rather than a mock -- a real click on a
	real button, arbitrated by the real PointerRouter, read back off the
	real tree the host holds.

	    cargo run -- test examples/aether/dialog-demo

	WHAT THIS PROVES that a screenshot alone cannot: that a second dialog
	stacks OVER the first without fighting it for the backdrop (both stay
	`Visible` at once), and that closing the TOPMOST one leaves the one
	underneath untouched -- the milestone's own "does the OLD trap
	deactivate while a new one is active" question, answered by driving the
	real ScreenStack/PointerRouter rather than assuming the composition.
]=]
local T = require("@testkit")
local ScreenStack = require("./roblox_packages/aether").ScreenStack

require("./dialog-demo")

local root = (DewTest :: any).Root
local pointer = (DewTest :: any).Pointer
local harness = (DewTest :: any)

--- The first descendant named `name`, depth-first -- the same helper
--- `contextmenu.test.luau` already proves out.
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

local function click(x: number, y: number)
	pointer.Move(x, y)
	pointer.Down(x, y)
	pointer.Up(x, y)
end

local function clickCenter(element: any)
	local position, size = element.AbsolutePosition, element.AbsoluteSize
	click(position.X + size.X / 2, position.Y + size.Y / 2)
end

local function dialogA(): any
	return find(root, "DialogA")
end

local function dialogB(): any
	return find(root, "DialogB")
end

local function settle()
	harness.Settle({ width = 420, height = 300 })
end

local function testDialogsStartClosed()
	settle()
	assert(dialogA() ~= nil, "the applet should have built Dialog A")
	assert(not dialogA().Visible, "a dialog nobody opened is closed")
	assert(dialogB() ~= nil, "the applet should have built Dialog B")
	assert(not dialogB().Visible, "a dialog nobody opened is closed")
end

local function testOpenButtonOpensDialogA()
	settle()
	local openButton = find(root, "OpenDialogButton")
	assert(openButton ~= nil, "the applet should draw its own open button")
	clickCenter(openButton)
	assert(dialogA().Visible, "clicking Open Dialog should open Dialog A")
end

local function testNestedButtonStacksDialogBOverDialogA()
	settle()
	local nestedButton = find(dialogA(), "OpenNestedButton")
	assert(nestedButton ~= nil, "Dialog A's own content should draw its nested-open button")
	clickCenter(nestedButton)
	assert(dialogB().Visible, "clicking Open nested dialog should open Dialog B")
	assert(dialogA().Visible, "Dialog A should still be open -- Modal is not Exclusive")
end

--- `.Visible` IS THE WRONG SIGNAL FOR "DID IT CLOSE" -- Dialog.luau's own
--- `Presence` deliberately keeps a dialog MOUNTED (`Visible = true`) through
--- its own 0.25s exit fade (`TransitionOrchestrator.Presets.FadeScale`),
--- which is correct, designed behaviour, not a bug: see Dialog.luau's own
--- "SHOULD THIS STILL BE IN THE TREE" comment. `DewTest.Settle` only
--- recomputes layout (`commit_geometry`, confirmed by reading
--- `host/src/main.rs`'s own `Settle` handler) -- it never advances a clock,
--- so it cannot let that fade complete, and asserting `.Visible` right after
--- a click would be testing whether an animation finished in zero elapsed
--- time. The real, immediate signal a click closes something is the
--- LOGICAL state -- `ScreenStack.IsOpen(id)` -- which `createDialog`'s own
--- `close()` flips synchronously from the same click, confirmed directly:
--- instrumenting this test showed `ScreenStack.IsOpen("dialog-b")` reading
--- `false` immediately after the close-button click, while `.Visible`
--- correctly still read `true` (mid-fade). Asserting `.Visible` here was
--- this test's own bug, not Dialog.luau's.
local function testClosingDialogBLeavesDialogAOpen()
	settle()
	local closeB = find(dialogB(), "CloseButton")
	assert(closeB ~= nil, "Dialog B should draw its own close button")
	clickCenter(closeB)
	assert(not ScreenStack.IsOpen("dialog-b")(), "Dialog B's own close button should close it")
	assert(ScreenStack.IsOpen("dialog-a")(), "closing the TOP dialog should not touch the one underneath")
end

local function testClickingOutsideDialogAsSurfaceClosesIt()
	settle()
	--- OUTSIDE THE SURFACE'S OWN 360x220 RECTANGLE, centred in a 420x300
	--- window (a 30px/40px margin) -- a point inside the surface would be
	--- OCCLUDED by it in render order and never reach the backdrop
	--- underneath, the same "render placement decides arbitration"
	--- PointerRouter's own suite already proves for an unrelated case.
	click(8, 8)
	assert(not ScreenStack.IsOpen("dialog-a")(), "clicking the visible backdrop margin should close Dialog A")
end

T.test("both dialogs start closed", testDialogsStartClosed, "math")
T.test("the open button opens Dialog A", testOpenButtonOpensDialogA, "math")
T.test("the nested button stacks Dialog B over Dialog A", testNestedButtonStacksDialogBOverDialogA, "math")
T.test("closing Dialog B leaves Dialog A open", testClosingDialogBLeavesDialogAOpen, "math")
T.test("clicking outside Dialog A's surface closes it", testClickingOutsideDialogAsSurfaceClosesIt, "math")

return T.run("dialog-demo")
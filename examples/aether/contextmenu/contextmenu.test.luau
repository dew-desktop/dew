--!strict
--!tier math
--[=[
	The menu opens where it was summoned and lands on the row it was aimed at.

	    cargo run -- test examples/aether/contextmenu

	IT REQUIRES THE APPLET, and the applet returns nothing. Requiring it runs it:
	it asks the host for a surface, builds under it, and ends. Everything below
	reads the tree the host holds and drives the pointer the host reports, which
	is the path a person takes.

	NOTHING IS MOUNTED TWICE. An earlier arrangement had the applet hand its
	framework back so a tool could mount the tree again under a recording copy and
	measure that. What it measured was the second mount.
]=]
local T = require("@testkit")

require("./contextmenu")

local root = (DewTest :: any).Root
local pointer = (DewTest :: any).Pointer
local harness = (DewTest :: any)

--- The first descendant named `name`, or nil.
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

local function chosen(): string
	local label = find(root, "Chosen")
	return if label then label.Text else "<no label>"
end

--- THE ROWS ALWAYS EXIST; the surface is what opens and closes. A test asserting
--- on the rows' presence would pass on a menu that never opened, because they are
--- built once and hidden with their parent.
local function menu(): any
	return find(root, "AetherContextMenu")
end

local function testARightClickOpensTheMenu()
	assert(menu() ~= nil, "the applet should have built a menu")
	assert(not menu().Visible, "a menu nobody summoned is not shown")

	pointer.Down(40, 30, 1)
	pointer.Up(40, 30, 1)

	assert(menu().Visible, "a right-click should open the menu")
end

local function testAPressLandsOnTheRowItWasAimedAt()
	assert(menu().Visible, "the menu should be open from the case above")

	--- LAYOUT FIRST. `AbsolutePosition` reads zero until something computes it,
	--- and a press at the origin lands on whichever row happens to be there.
	harness.Settle({ width = 240, height = 180 })

	local row = find(root, "Item_copy")
	assert(row ~= nil, "an open menu draws its rows")

	--- FOUND IN THE TREE RATHER THAN COMPUTED. A test that works out where a row
	--- ought to be agrees with its own arithmetic; this one agrees with the
	--- layout, so a row that moves is a failure rather than a silent pass on
	--- whatever is at the coordinate.
	local position, size = row.AbsolutePosition, row.AbsoluteSize
	local x = position.X + size.X / 2
	local y = position.Y + size.Y / 2

	pointer.Move(x, y)
	pointer.Down(x, y)
	pointer.Up(x, y)

	assert(chosen() == "chosen: copy", `the press should land on Copy, drew {chosen()}`)
end

local function testChoosingARowClosesTheMenu()
	assert(not menu().Visible, "choosing a row should close the menu")
end

T.test("a right-click opens the menu where it was pressed", testARightClickOpensTheMenu, "math")
T.test("a press lands on the row it was aimed at", testAPressLandsOnTheRowItWasAimedAt, "math")
T.test("choosing a row closes the menu", testChoosingARowClosesTheMenu, "math")

return T.run("contextmenu")

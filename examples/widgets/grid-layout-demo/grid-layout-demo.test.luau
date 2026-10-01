--!strict
--!tier math
--[=[
	Drives grid-layout-demo through the real host: a click on a real toolbar
	button or tile, delivered by DewTest.Pointer and routed by the host's own hit
	test, then the launcher's geometry read back off the tree the host laid out.

	    cargo run -- test examples/widgets/grid-layout-demo

	The suites run in order and share one applet, the same way a person clicks
	through it. Every position asserted here follows the rule the engine showed
	for the same arrangement in Studio, named beside each assertion by its
	conformance case.
]=]
local T = require("@testkit")

require("./grid-layout-demo")

local harness = (DewTest :: any)
local root = harness.Root
local pointer = harness.Pointer

local NEAR = 0.01
--- The launcher's UIPadding, which the grid's cells start inside.
local INSET = 8

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

local function settle()
	harness.Settle({ width = 480, height = 380 })
end

--- Click the middle of the element named `name`, and lay the tree out again so
--- the next read sees the result.
local function press(name: string)
	settle()
	local element = find(root, name)
	assert(element ~= nil, `the applet should draw {name}`)
	local x = element.AbsolutePosition.X + element.AbsoluteSize.X / 2
	local y = element.AbsolutePosition.Y + element.AbsoluteSize.Y / 2
	pointer.Move(x, y)
	pointer.Down(x, y)
	pointer.Up(x, y)
	settle()
end

local function near(a: number, b: number, what: string)
	assert(math.abs(a - b) <= NEAR, `{what}: expected {b}, got {a}`)
end

--- Where the tile named `name` sits, relative to the first cell of the
--- launcher, and how big it is.
local function at(name: string): (number, number, number, number)
	local launcher = find(root, "Launcher")
	local tile = find(launcher, name)
	assert(tile ~= nil, `the launcher should hold {name}`)
	local origin = launcher.AbsolutePosition
	return tile.AbsolutePosition.X - origin.X - INSET,
		tile.AbsolutePosition.Y - origin.Y - INSET,
		tile.AbsoluteSize.X,
		tile.AbsoluteSize.Y
end

--- The tile named `name` is the cell at (`x`, `y`) from the first, `w` by `h`.
local function cell(name: string, x: number, y: number, w: number, h: number)
	local tx, ty, tw, th = at(name)
	near(tx, x, `{name} x`)
	near(ty, y, `{name} y`)
	near(tw, w, `{name} width`)
	near(th, h, `{name} height`)
end

local function status(): string
	return find(root, "Status").Text
end

local function says(text: string)
	local line = status()
	assert(string.find(line, text, 1, true), `the status line should say "{text}", got "{line}"`)
end

local function testAtRestFiveTilesFitARow()
	settle()
	-- uigridlayout_defaults: cells fill across, a line holding as many as fit.
	-- The launcher's content box is 432 wide, and 72 cells 8 apart fit five.
	cell("Mail", 0, 0, 72, 72)
	cell("Calendar", 80, 0, 72, 72)
	cell("Notes", 320, 0, 72, 72)
	cell("Maps", 0, 80, 72, 72)
	cell("Settings", 160, 80, 72, 72)
	local grid = find(root, "Grid")
	near(grid.AbsoluteCellCount.X, 5, "five columns")
	near(grid.AbsoluteCellCount.Y, 2, "two rows")
	near(grid.AbsoluteContentSize.X, 5 * 72 + 4 * 8, "the block's width")
	near(grid.AbsoluteContentSize.Y, 2 * 72 + 8, "the block's height")
	says("tiles 72 x 72")
	says("columns auto")
	says("selected none")
end

local function testIconsStaySquare()
	-- A 72 tile offers its icon 56 by 42; the aspect ratio of 1 keeps the
	-- largest square inside that, the rule
	-- uigridlayout_aspect_ratio_constraint_overrides_the_cell shows in a cell.
	local icon = find(find(root, "Mail"), "Icon")
	near(icon.AbsoluteSize.X, 42, "the icon's width")
	near(icon.AbsoluteSize.Y, 42, "the icon's height")
end

local function testAClickSelectsTheTileUnderIt()
	press("Photos")
	says("selected Photos")
	near(find(find(root, "Photos"), "Outline").Transparency, 0, "Photos is outlined")
	near(find(find(root, "Mail"), "Outline").Transparency, 1, "Mail is not")
	press("Maps")
	says("selected Maps")
end

local function testColumnsCapsARowBeforeTheWidthDoes()
	press("ColumnsButton")
	says("columns 4")
	-- uigridlayout_fill_direction_max_cells: four to a row although five fit.
	cell("Photos", 240, 0, 72, 72)
	cell("Notes", 0, 80, 72, 72)
	cell("Settings", 240, 80, 72, 72)
	near(find(root, "Grid").AbsoluteCellCount.X, 4, "four columns")
end

local function testCornerRunsTheOrderFromTheBlocksCorner()
	press("CornerButton")
	says("from TopRight")
	-- uigridlayout_start_corner_top_right: the first cell is at the right of
	-- the block, not of the launcher, and the next is to its left.
	cell("Mail", 240, 0, 72, 72)
	cell("Calendar", 160, 0, 72, 72)
	cell("Notes", 240, 80, 72, 72)
	-- The hit test reads the same placement: a click where Mail now is
	-- selects Mail.
	press("Mail")
	says("selected Mail")

	press("CornerButton")
	says("from BottomRight")
	-- uigridlayout_start_corner_bottom_right.
	cell("Mail", 240, 80, 72, 72)
	cell("Calendar", 160, 80, 72, 72)
	cell("Notes", 240, 0, 72, 72)
end

local function testFillVerticalGoesDownFirst()
	press("CornerButton") -- BottomLeft
	press("CornerButton") -- TopLeft
	says("from TopLeft")
	press("FillButton")
	says("fill Vertical")
	-- uigridlayout_fill_direction_vertical: down before across. The content
	-- box is 232 tall, so three 72 cells fit a column.
	cell("Mail", 0, 0, 72, 72)
	cell("Calendar", 0, 80, 72, 72)
	cell("Music", 0, 160, 72, 72)
	cell("Photos", 80, 0, 72, 72)
	local grid = find(root, "Grid")
	near(grid.AbsoluteCellCount.X, 3, "three columns")
	near(grid.AbsoluteCellCount.Y, 3, "three rows")
end

local function testSizeChangesHowManyFit()
	press("FillButton")
	says("fill Horizontal")
	press("SizeButton")
	says("tiles 56 x 56")
	-- Still capped at four a row; the smaller cells move closer.
	cell("Calendar", 64, 0, 56, 56)
	cell("Notes", 0, 64, 56, 56)

	press("SizeButton")
	says("tiles 112 x 56")
	-- 112 cells 8 apart fit three to a row of 432, under the cap of four.
	cell("Music", 240, 0, 112, 56)
	cell("Photos", 0, 64, 112, 56)
	-- A wide cell still holds a square icon.
	local icon = find(find(root, "Photos"), "Icon")
	near(icon.AbsoluteSize.X, icon.AbsoluteSize.Y, "the icon stays square")
	near(find(root, "Grid").AbsoluteCellCount.X, 3, "three columns")
end

T.test("at rest five tiles fit a row", testAtRestFiveTilesFitARow, "math")
T.test("icons stay square", testIconsStaySquare, "math")
T.test("a click selects the tile under it", testAClickSelectsTheTileUnderIt, "math")
T.test("Columns caps a row before the width does", testColumnsCapsARowBeforeTheWidthDoes, "math")
T.test("Corner runs the order from the block's corner", testCornerRunsTheOrderFromTheBlocksCorner, "math")
T.test("Fill Vertical goes down first", testFillVerticalGoesDownFirst, "math")
T.test("Size changes how many fit", testSizeChangesHowManyFit, "math")

return T.run("grid-layout-demo")

--!strict
--!tier math
--[=[
	Drives flex-list-demo through the real host: a click on a real toolbar
	button, delivered by DewTest.Pointer and routed by the host's own hit test,
	then the chips' geometry read back off the tree the host laid out.

	    cargo run -- test examples/widgets/flex-list-demo

	The suites run in order and share one applet, the same way a person clicks
	through it: each one starts from where the previous one left the layout.
	The geometry each asserts is the behaviour the engine showed for the same
	property in Studio, not a number copied from this host's own output.
]=]
local T = require("@testkit")

require("./flex-list-demo")

local harness = (DewTest :: any)
local root = harness.Root
local pointer = harness.Pointer

local TAGS = { "layout", "flex", "Luau", "accessibility", "wrapping", "UI", "design systems", "Roblox", "desktop" }
local GAP = 6
local NEAR = 0.5

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
	harness.Settle({ width = 440, height = 320 })
end

--- Click the middle of the toolbar button named `name`, and lay the tree out
--- again so the next read sees the result.
local function press(name: string)
	settle()
	local button = find(root, name)
	assert(button ~= nil, `the applet should draw a {name} button`)
	local x = button.AbsolutePosition.X + button.AbsoluteSize.X / 2
	local y = button.AbsolutePosition.Y + button.AbsoluteSize.Y / 2
	pointer.Move(x, y)
	pointer.Down(x, y)
	pointer.Up(x, y)
	settle()
end

type Box = { name: string, x: number, y: number, w: number, h: number, right: number, bottom: number }

local function box(element: any): Box
	local p, s = element.AbsolutePosition, element.AbsoluteSize
	return { name = element.Name, x = p.X, y = p.Y, w = s.X, h = s.Y, right = p.X + s.X, bottom = p.Y + s.Y }
end

--- The tray's content box: its own rectangle inside its 8 pixel UIPadding.
local function tray(): Box
	local t = box(find(root, "Tray"))
	return { name = "content", x = t.x + 8, y = t.y + 8, w = t.w - 16, h = t.h - 16, right = t.right - 8, bottom = t.bottom - 8 }
end

--- Every chip, in tag order, grouped into the lines the layout made: a chip
--- that starts no further right than the one before it starts a new line.
local function lines(): { { Box } }
	local result: { { Box } } = {}
	local previous: Box? = nil
	for _, tag in TAGS do
		local chip = box(find(root, tag))
		if previous == nil or chip.x <= previous.x then
			table.insert(result, {})
		end
		table.insert(result[#result], chip)
		previous = chip
	end
	return result
end

local function near(a: number, b: number, what: string)
	assert(math.abs(a - b) <= NEAR, `{what}: expected {b}, got {a}`)
end

local function settingsText(): string
	return find(root, "Settings").Text
end

local widths: { [string]: number } = {}

local function testAtRestTheChipsRunOffTheTrayOnOneLine()
	settle()
	local content = tray()
	local all = lines()
	assert(#all == 1, `without Wraps there is one line, got {#all}`)
	local line = all[1]
	near(line[1].x, content.x, "the first chip starts at the tray's left edge")
	for i, chip in line do
		widths[chip.name] = chip.w
		near(chip.y, content.y, `{chip.name} sits at the top of the line`)
		if i > 1 then
			near(chip.x, line[i - 1].right + GAP, `{chip.name} follows the chip before it`)
		end
	end
	assert(line[#line].right > content.right, "the last chip should run past the tray's right edge")
	assert(string.find(settingsText(), "Wraps off", 1, true), "the settings line should say Wraps is off")
end

local function testWrapsMovesTheOverflowOntoNewLines()
	press("Wraps")
	local content = tray()
	local all = lines()
	assert(#all >= 2, `Wraps should make a second line, got {#all}`)
	local first = all[1]
	local tallest = 0
	for _, chip in first do
		tallest = math.max(tallest, chip.h)
		assert(chip.right <= content.right + NEAR, `{chip.name} should fit inside the tray`)
	end
	-- A wrapped line starts at the left edge, one line height and the Padding
	-- below the line before it.
	near(all[2][1].x, content.x, "the second line starts at the left edge")
	near(all[2][1].y, content.y + tallest + GAP, "the second line sits below the first by the Padding")
	assert(string.find(settingsText(), "Wraps on", 1, true), "the settings line should say Wraps is on")
end

local function testFillWidensEveryChipOnALineByTheSameAmount()
	press("HorizontalFlex")
	local content = tray()
	for _, line in lines() do
		near(line[1].x, content.x, `{line[1].name} starts the line at the left edge`)
		near(line[#line].right, content.right, `{line[#line].name} ends the line at the right edge`)
		local grew = line[1].w - widths[line[1].name]
		assert(grew > 0, `{line[1].name} should have grown`)
		for _, chip in line do
			near(chip.w - widths[chip.name], grew, `{chip.name} grows by the same share`)
		end
	end
	assert(string.find(settingsText(), "flex Fill", 1, true), "the settings line should say Fill")
end

local function testSpaceBetweenPutsTheRoomBetweenTheChips()
	press("HorizontalFlex") -- SpaceAround
	press("HorizontalFlex") -- SpaceBetween
	local content = tray()
	for _, line in lines() do
		for _, chip in line do
			near(chip.w, widths[chip.name], `{chip.name} keeps its own width`)
		end
		if #line > 1 then
			near(line[1].x, content.x, `{line[1].name} is on the left edge`)
			near(line[#line].right, content.right, `{line[#line].name} is on the right edge`)
		end
	end
	assert(string.find(settingsText(), "flex SpaceBetween", 1, true), "the settings line should say SpaceBetween")
end

--- The line holding the featured chip, which is taller than the rest.
local function featuredLine(): { Box }
	for _, line in lines() do
		for _, chip in line do
			if chip.name == "flex" then
				return line
			end
		end
	end
	error("no line holds the flex chip")
end

local function testLineAlignCenterLevelsTheChipsOnTheTallOne()
	press("LineAlign") -- Start
	press("LineAlign") -- Center
	local line = featuredLine()
	local centre: number? = nil
	for _, chip in line do
		if chip.name == "flex" then
			centre = chip.y + chip.h / 2
		end
	end
	assert(centre ~= nil)
	for _, chip in line do
		near(chip.y + chip.h / 2, centre :: number, `{chip.name} is centred on the line`)
	end
end

local function testStretchMakesALineAsTallAsItsTallestChip()
	press("LineAlign") -- End
	press("LineAlign") -- Stretch
	local line = featuredLine()
	for _, chip in line do
		near(chip.h, 36, `{chip.name} stretches to the featured chip's height`)
	end
	assert(string.find(settingsText(), "line Stretch", 1, true), "the settings line should say Stretch")
end

local function testAlignYBottomDropsTheBlockOfLines()
	press("AlignY") -- Center
	press("AlignY") -- Bottom
	local content = tray()
	local all = lines()
	local last = all[#all]
	near(last[1].bottom, content.bottom, "the last line sits on the tray's bottom edge")
	assert(all[1][1].y < last[1].y, "the first line stays above the last")
end

T.test("at rest the chips run off the tray on one line", testAtRestTheChipsRunOffTheTrayOnOneLine, "math")
T.test("Wraps moves the overflow onto new lines", testWrapsMovesTheOverflowOntoNewLines, "math")
T.test("Fill widens every chip on a line by the same amount", testFillWidensEveryChipOnALineByTheSameAmount, "math")
T.test("SpaceBetween puts the room between the chips", testSpaceBetweenPutsTheRoomBetweenTheChips, "math")
T.test("line align Center levels the chips on the tall one", testLineAlignCenterLevelsTheChipsOnTheTallOne, "math")
T.test("Stretch makes a line as tall as its tallest chip", testStretchMakesALineAsTallAsItsTallestChip, "math")
T.test("Align Y Bottom drops the block of lines", testAlignYBottomDropsTheBlockOfLines, "math")

return T.run("flex-list-demo")

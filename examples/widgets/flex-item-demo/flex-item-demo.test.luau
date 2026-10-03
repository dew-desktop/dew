--!strict
--!tier math
--[=[
	Drives flex-item-demo through the real host: a click on a real toolbar
	button, delivered by DewTest.Pointer and routed by the host's own hit test,
	then the pane's geometry read back off the tree the host laid out.

	    cargo run -- test examples/widgets/flex-item-demo

	The suites run in order and share one applet, the same way a person clicks
	through it. Every width asserted here is what the engine gave the same
	arrangement in Studio, named beside each assertion by its conformance case.
]=]
local T = require("@testkit")

require("./flex-item-demo")

local harness = (DewTest :: any)
local root = harness.Root
local pointer = harness.Pointer

local GAP = 6
local NEAR = 0.01

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
	harness.Settle({ width = 480, height = 320 })
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

type Box = { x: number, y: number, w: number, h: number, right: number }

local function box(name: string): Box
	local element = find(root, name)
	assert(element ~= nil, `the applet should draw {name}`)
	local p, s = element.AbsolutePosition, element.AbsoluteSize
	return { x = p.X, y = p.Y, w = s.X, h = s.Y, right = p.X + s.X }
end

local function near(a: number, b: number, what: string)
	assert(math.abs(a - b) <= NEAR, `{what}: expected {b}, got {a}`)
end

local function settingsText(): string
	return find(root, "Settings").Text
end

--- The composer row, left to right, and the pane it sits in.
local function composer(): (Box, Box, Box, Box, Box)
	return box("Pane"), box("Attach"), box("Field"), box("Chip"), box("Send")
end

--- Each child follows the one before it by the list's Padding.
local function packed(attach: Box, field: Box, chip: Box, send: Box)
	near(field.x, attach.right + GAP, "the field follows the + button")
	near(chip.x, field.right + GAP, "the chip follows the field")
	near(send.x, chip.right + GAP, "Send follows the chip")
end

local function testAtRestTheFieldTakesTheSpareWidth()
	settle()
	local pane, attach, field, chip, send = composer()
	near(pane.w, 440, "the pane starts 440 wide")
	near(attach.x, pane.x, "the + button starts the row")
	packed(attach, field, chip, send)
	-- uiflexitem_grow_takes_the_free_space: a Grow child takes all of the
	-- 160 the row has spare, and its plain neighbours keep their own widths.
	near(field.w, 80 + 160, "the field grows by the row's spare 160")
	near(chip.w, 90, "the chip keeps its width")
	near(send.w, 60, "Send, with GrowRatio 0, keeps its width")
	near(send.right, pane.right, "Send ends the row at the pane's edge")
	assert(string.find(settingsText(), "field Grow", 1, true), "the settings line should say the field is Grow")
end

local function testTheSidebarStopsAtItsMaxSize()
	local pane, sidebar, messages = box("Pane"), box("Sidebar"), box("Messages")
	-- uisizeconstraint_clamped_size_advances_a_list: 40% of 440 is 176, the
	-- MaxSize holds it at 140, and the list advances by the clamped 140.
	near(sidebar.w, 140, "the sidebar stops at its MaxSize")
	near(messages.x, sidebar.right + GAP, "the messages follow the clamped sidebar")
	-- uiflexitem_grow_basis_is_the_resolved_scale_size: the messages grow from
	-- their own basis into whatever the sidebar left.
	near(messages.right, pane.right, "the messages fill the rest of the pane")
	-- The messages are 294 wide and padded 8 a side; each bubble would be 278
	-- and its own MaxSize caps it.
	near(box("Bubble1").w, 240, "a bubble stops at its MaxSize")
end

local function testGrowRatioSplitsTheSpareWidthTwoToOne()
	press("FieldMode") -- Custom, GrowRatio 2
	press("SendRatio") -- GrowRatio 1
	local pane, attach, field, chip, send = composer()
	packed(attach, field, chip, send)
	-- uiflexitem_custom_grow_ratio_weights_the_split: the same 160 split two
	-- to one, by ratio and not by width.
	near(field.w, 80 + 160 * 2 / 3, "the field takes two shares")
	near(send.w, 60 + 160 / 3, "Send takes one share")
	near(chip.w, 90, "the chip still keeps its width")
	near(send.right, pane.right, "the row still ends at the pane's edge")
	local text = settingsText()
	assert(string.find(text, "GrowRatio 2", 1, true), `the settings line should show the field's ratio, got {text}`)
	assert(string.find(text, "Send GrowRatio 1", 1, true), `the settings line should show Send's ratio, got {text}`)
end

local function testAtItsOwnWidthNothingFlexes()
	press("Narrower") -- 360
	press("Narrower") -- 280
	local pane, attach, field, chip, send = composer()
	near(pane.w, 280, "two clicks take the pane to 280")
	packed(attach, field, chip, send)
	near(field.w, 80, "the field is at its basis")
	near(chip.w, 90, "the chip is at its basis")
	near(send.w, 60, "Send is at its basis")
	near(send.right, pane.right, "the bases fill the pane exactly")
end

local function testTheChipShrinksToItsMinSize()
	press("Narrower") -- 200
	local pane, attach, field, chip, send = composer()
	near(pane.w, 200, "a third click takes the pane to 200")
	packed(attach, field, chip, send)
	-- uiflexitem_shrink_stops_at_min_size: the chip is the only child that
	-- shrinks, and its MinSize stops it at 64 with the overflow unabsorbed.
	near(chip.w, 64, "the chip stops at its MinSize")
	near(field.w, 80, "the field, Custom with ShrinkRatio 0, gives nothing back")
	near(send.w, 60, "Send gives nothing back")
	near(send.right, pane.right + 54, "the row runs 54 past the pane's edge")

	-- Under its MaxSize now, the sidebar is 40% of the pane again, and the
	-- messages grow from their 100 into the 14 left over.
	local sidebar, messages = box("Sidebar"), box("Messages")
	near(sidebar.w, 80, "the sidebar is 40% of 200")
	near(messages.w, 114, "the messages take the 14 left over")
end

local function testWiderGivesTheChipItsWidthBack()
	press("Wider") -- 280
	press("Wider") -- 360
	press("Wider") -- 440
	local pane, _, _, chip, send = composer()
	near(pane.w, 440, "three clicks take the pane back to 440")
	near(chip.w, 90, "the chip is back to 90")
	near(send.right, pane.right, "the row fits the pane again")
	near(box("Sidebar").w, 140, "the sidebar is held at 140 again")
end

T.test("at rest the field takes the spare width", testAtRestTheFieldTakesTheSpareWidth, "math")
T.test("the sidebar stops at its MaxSize", testTheSidebarStopsAtItsMaxSize, "math")
T.test("GrowRatio splits the spare width two to one", testGrowRatioSplitsTheSpareWidthTwoToOne, "math")
T.test("at its own width nothing flexes", testAtItsOwnWidthNothingFlexes, "math")
T.test("the chip shrinks to its MinSize", testTheChipShrinksToItsMinSize, "math")
T.test("Wider gives the chip its width back", testWiderGivesTheChipItsWidthBack, "math")

return T.run("flex-item-demo")

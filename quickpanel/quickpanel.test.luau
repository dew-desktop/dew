--!strict
--!tier math
--[=[
	The quick panel against a fake `dew.Library`.

	`dew test` never sets a `dew` global, so this file sets one before it
	requires the panel. The fake keeps its own list, records every
	`SetEnabled`, and fires `OnChange` synchronously the way the host's
	does once a change lands. `Uninstall` and `Launch` fail the suite if
	the panel ever calls them: neither belongs to this surface.

		cargo run -- test quickpanel
]=]

local T = require("@testkit")

type Entry = { id: string, name: string, description: string, enabled: boolean, running: boolean }

local entries: { Entry } = {
	{ id = "calculator", name = "Calculator", description = "", enabled = true, running = true },
	{ id = "workspace", name = "Workspace", description = "", enabled = false, running = false },
	{
		id = "timetracker",
		name = "Time Tracker & Pomodoro HUD with a name far too long for any card",
		description = "",
		enabled = true,
		running = false,
	},
}

local onChange: (() -> ())? = nil
local calls: { { id: string, enabled: boolean } } = {}
local failNext: string? = nil

local fakeLibrary = {
	List = function(): { Entry }
		local copy = {}
		for i, entry in entries do
			copy[i] = table.clone(entry)
		end
		return copy
	end,
	OnChange = function(callback: () -> ())
		onChange = callback
	end,
	SetEnabled = function(id: string, enabled: boolean)
		table.insert(calls, { id = id, enabled = enabled })
		if failNext then
			local message = failNext
			failNext = nil
			return { ok = false, error = message }
		end
		for _, entry in entries do
			if entry.id == id then
				entry.enabled = enabled
				entry.running = enabled
			end
		end
		if onChange then
			onChange()
		end
		return { ok = true }
	end,
	Uninstall = function(_id: string)
		error("the quick panel must never uninstall an applet")
	end,
	Launch = function(_id: string)
		error("the quick panel starts applets through SetEnabled, never Launch")
	end,
}

dew = ({ Library = fakeLibrary } :: any)

require("./quickpanel")

local root = (DewTest :: any).Root
local pointer = (DewTest :: any).Pointer
local harness = (DewTest :: any)

local function find(from: any, name: string): any
	for _, child in from:GetChildren() do
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
	harness.Settle({ width = 320, height = 420 })
end

local function cards(): { any }
	local out = {}
	for _, child in find(root, "Grid"):GetChildren() do
		if child.Name == "Card" then
			table.insert(out, child)
		end
	end
	return out
end

local function cardFor(name: string): any
	for _, c in cards() do
		if find(c, "Name").Text == name then
			return c
		end
	end
	return nil
end

local function isOn(c: any): boolean
	local layout = find(find(c, "Toggle"), "UIListLayout")
	local right = layout.HorizontalAlignment == Enum.HorizontalAlignment.Right
	local running = find(find(c, "State"), "Label").Text == "Running"
	assert(right == running, "the switch's knob and the state label must agree")
	return running
end

local function clickCenter(element: any)
	settle()
	local position, size = element.AbsolutePosition, element.AbsoluteSize
	local x, y = position.X + size.X / 2, position.Y + size.Y / 2
	pointer.Move(x, y)
	pointer.Down(x, y)
	pointer.Up(x, y)
end

T.test("one card per installed applet, with the count in the header", function()
	settle()
	assert(#cards() == 3, `expected 3 cards, found {#cards()}`)
	assert(find(root, "Count").Text == "3 installed, 1 running", find(root, "Count").Text)
	assert(not find(root, "Empty").Visible, "the empty notice shows only with nothing installed")
end, "math")

T.test("a card shows its state on the switch and the label", function()
	settle()
	assert(isOn(cardFor("Calculator")), "Calculator is running")
	assert(not isOn(cardFor("Workspace")), "Workspace is stopped")
end, "math")

T.test("two cards share a row at the default size", function()
	settle()
	local a, b = cardFor("Calculator"), cardFor("Workspace")
	assert(a.AbsolutePosition.Y == b.AbsolutePosition.Y, "the first two cards sit on one row")
	assert(b.AbsolutePosition.X > a.AbsolutePosition.X, "the second card is to the right of the first")
	local shelf = find(root, "Shelf")
	local right = b.AbsolutePosition.X + b.AbsoluteSize.X
	assert(right <= shelf.AbsolutePosition.X + shelf.AbsoluteSize.X, "a card never runs past the shelf")
end, "math")

T.test("a long name stays inside its card", function()
	settle()
	local c = cardFor("Time Tracker & Pomodoro HUD with a name far too long for any card")
	local name = find(c, "Name")
	assert(name.TextTruncate == Enum.TextTruncate.AtEnd, "names truncate rather than wrap")
	assert(
		name.AbsolutePosition.X + name.AbsoluteSize.X <= c.AbsolutePosition.X + c.AbsoluteSize.X,
		"the name's box ends inside the card"
	)
	assert(not name.TextFits, "this name is too long to fit, so it is the cut one")
end, "math")

T.test("the switch starts a stopped applet and the card turns on", function()
	local before = #calls
	clickCenter(find(cardFor("Workspace"), "Toggle"))
	assert(#calls == before + 1, "one click is one SetEnabled call")
	local call = calls[#calls]
	assert(call.id == "workspace" and call.enabled == true, `called SetEnabled({call.id}, {call.enabled})`)
	settle()
	assert(isOn(cardFor("Workspace")), "the Workspace card now reads Running")
	assert(find(root, "Count").Text == "3 installed, 2 running", find(root, "Count").Text)
end, "math")

T.test("the switch stops a running applet and the card turns off", function()
	local before = #calls
	clickCenter(find(cardFor("Calculator"), "Toggle"))
	assert(#calls == before + 1, "one click is one SetEnabled call")
	local call = calls[#calls]
	assert(call.id == "calculator" and call.enabled == false, `called SetEnabled({call.id}, {call.enabled})`)
	settle()
	assert(not isOn(cardFor("Calculator")), "the Calculator card now reads Stopped")
end, "math")

T.test("a refused switch says why and leaves the card as it was", function()
	failNext = "could not reach the Dew service"
	clickCenter(find(cardFor("Workspace"), "Toggle"))
	settle()
	local notice = find(root, "Notice")
	assert(notice.Visible and notice.Text == "could not reach the Dew service", notice.Text)
	assert(isOn(cardFor("Workspace")), "a failed call leaves the card on")

	clickCenter(find(cardFor("Workspace"), "Toggle"))
	assert(not find(root, "Notice").Visible, "the next successful action clears the notice")
end, "math")

T.test("a change made elsewhere reaches the panel", function()
	table.insert(entries, { id = "nameplate", name = "Nameplate", description = "", enabled = true, running = true })
	assert(onChange ~= nil, "the panel subscribes to Library.OnChange")
	;(onChange :: () -> ())()
	settle()
	assert(cardFor("Nameplate") ~= nil, "an applet installed elsewhere gets a card")

	table.insert(entries, { id = "quickpanel", name = "Quick panel", description = "", enabled = true, running = true })
	;(onChange :: () -> ())()
	settle()
	assert(cardFor("Quick panel") == nil, "the panel never lists itself")

	table.clear(entries)
	;(onChange :: () -> ())()
	settle()
	assert(#cards() == 0, "an empty library has no cards")
	assert(find(root, "Empty").Visible, "and says so")
	assert(find(root, "Count").Text == "0 installed, 0 running", find(root, "Count").Text)
end, "math")

return T.run("quickpanel")

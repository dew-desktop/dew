--!strict
--!tier math
--[=[
	Drives the system monitor through the real host, against this machine's
	own readings: whatever `desktop.System` says, the tree has to agree.

	    cargo run -- test examples/system/monitor
]=]
local T = require("@testkit")

require("./monitor")

local harness = (DewTest :: any)
local root = harness.Root
local pointer = harness.Pointer
local System = (desktop :: any).System

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
	harness.Settle({ width = 340, height = 800 })
end

local function press(name: string)
	settle()
	local element = find(root, name)
	assert(element ~= nil, `the monitor should draw {name}`)
	local x = element.AbsolutePosition.X + element.AbsoluteSize.X / 2
	local y = element.AbsolutePosition.Y + element.AbsoluteSize.Y / 2
	pointer.Move(x, y)
	pointer.Down(x, y)
	pointer.Up(x, y)
	settle()
end

local function testEveryCoreIsOnATile()
	-- Up to sixteen threads, one tile each; past that, the same sixteen tiles
	-- each average a run of neighbours. Either way every core is on a tile and
	-- the tree holds sixteen whatever the machine.
	local cores = #System.Cpu().cores
	local group = math.ceil(cores / 16)
	local grid = find(root, "Cores")
	local built, shown = 0, 0
	for _, child in ipairs(grid:GetChildren()) do
		if string.match(child.Name, "^Tile%d+$") then
			built += 1
			if child.Visible then
				shown += 1
			end
		end
	end
	assert(built == 16, `the grid always holds sixteen tiles, found {built}`)
	assert(shown * group >= cores, `{shown} tiles of {group} cannot cover {cores} cores`)
	assert((shown - 1) * group < cores, `{shown} tiles of {group} leaves one empty for {cores} cores`)
end

local function testSixtyBarsShareTheGraphEvenly()
	settle()
	local graph = find(find(root, "Graph"), "Bars")
	local widths = {}
	for _, child in ipairs(graph:GetChildren()) do
		if string.match(child.Name, "^Bar%d+$") then
			table.insert(widths, child.AbsoluteSize.X)
		end
	end
	assert(#widths == 60, `a minute of samples is sixty bars, found {#widths}`)
	local low, high = math.huge, 0
	for _, w in widths do
		low, high = math.min(low, w), math.max(high, w)
	end
	assert(low > 1, `Fill should widen every bar past the 1px it asks for, narrowest is {low}`)
	assert(high - low <= 1, `the bars should share the width evenly, {low} to {high}`)
end

local function testMemoryIsSplitByItsRatio()
	settle()
	local used = find(find(root, "MemoryBar"), "Used").AbsoluteSize.X
	local free = find(find(root, "MemoryBar"), "Free").AbsoluteSize.X
	local m = System.Memory()
	local expected = m.used / m.total
	local drawn = used / (used + free)
	-- The reading and the tree come from the same sample, so the only error is
	-- rounding to pixels, which on a bar this short is a few percent at most.
	assert(math.abs(drawn - expected) < 0.05, `used should be {expected} of the bar, drew {drawn}`)
end

local function testTheSwitchResortsByMemory()
	press("MEM")
	local value = find(find(root, "Process1"), "Value").Text
	assert(string.find(value, "B$") ~= nil, `sorted by memory, the first row shows a size, got {value}`)
	assert((desktop :: any).Storage.get("monitor.processes") == "memory", "the choice is kept")

	press("CPU")
	value = find(find(root, "Process1"), "Value").Text
	assert(string.find(value, "%%$") ~= nil, `sorted by CPU, the first row shows a share, got {value}`)
	assert((desktop :: any).Storage.get("monitor.processes") == "cpu", "the choice is kept")
end

T.test("every core is on a tile", testEveryCoreIsOnATile, "math")
T.test("sixty bars share the graph evenly", testSixtyBarsShareTheGraphEvenly, "math")
T.test("memory is split by its ratio", testMemoryIsSplitByItsRatio, "math")
T.test("the switch re-sorts by memory", testTheSwitchResortsByMemory, "math")

return T.run("monitor")

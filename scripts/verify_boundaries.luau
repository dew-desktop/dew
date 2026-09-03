--!strict
--[[
	verify_boundaries -- Dew owns the host, and the host is Rust.

	    lune run scripts/verify_boundaries.luau

	WHY (ADR-004, .artifacts/project/milestones/1_host_owned_datamodel/decisions)
	Three repositories, three jobs:

	    Aether    a headless UI framework. Luau only. Runs on any host adhering to
	              the DataModel Standard, implements no DataModel of its own.
	    Dew       a desktop applet platform with a NATIVE DataModel adhering to
	              that standard, and the whole rendering stack under it.
	    dew-cli   headless renders for CI, on a machine with no display.

	Aether's Rust workspace -- the display list, painter, rasteriser and window --
	is Dew's rendering stack living in the framework's repository, left there from
	when the zune experiment and the CLI needed it. None of it reaches a Luau
	consumer, and after aether-cli retires Dew is its only dependent. It moves here.

	A BOUNDARY WITH NOTHING CHECKING IT IS A BOUNDARY NOBODY FOLLOWS. This project
	has found that class five times in one milestone: `rbx_reflection` sat in
	Cargo.toml described in the present tense with no importer, the manifest
	accepted hotkeys nothing bound, `suites.luau` claimed to be read by a runner
	that had stopped reading it, the inventory gate claimed a vacuity check that
	did not exist here, and the surface census counted by name and reported 100%.
	So the rule is a script, and CI runs it.

	THE ALLOWLIST IS THE MIGRATION. A dependency not on it fails on the commit that
	adds it; the ones on it are named, counted, and have to be deleted from the
	list to be resolved. Emptying it is what finishing ADR-004 looks like from here.
]]

local fs = require("@lune/fs")
local process = require("@lune/process")

local failures = 0
local function fail(msg: string)
	failures += 1
	print("FAIL  " .. msg)
end

--- Crates still borrowed from the Aether repository. ADR-004 moves them here.
---
--- `aether_runtime` splits on the way in: the painter, the rasteriser hookup and
--- the VM hosting become Dew's outright, while `session.rs`, `driver.rs` and
--- `frame.rs`'s decoding half retire with `Headless.luau` -- they exist to consume
--- what `Live.luau` emits, and an Aether guest on Dew will build DataModel
--- instances instead, exactly as it does on Roblox.
---
--- `aether_raster`'s `gpu` feature gate must survive the move intact. It is what
--- lets one crate serve a CPU snapshot and a GPU swapchain, with the measurement
--- recorded in its manifest, and flattening it during a move would be a silent
--- regression of exactly the kind this script exists to catch.
local BORROWED_CRATES = {
	"aether_runtime",
	"aether_raster",
	"aether_window",
}

--- Host files still reaching for a borrowed crate, and what for.
local BORROWED_USE = {
	["host/src/main.rs"] = "the window, the painter and the frame loop",
	["host/src/mods.rs"] = "the Luau VM and module installation",
	["host/src/datamodel/render.rs"] = "Frame and Node as the render IR",
	["host/src/surface.rs"] = "the Surface enum a window is opened from",
}

--- Rust with its comments removed.
---
--- LOAD-BEARING, NOT TIDINESS. On this script's first run `host/src/tray.rs` came
--- back as borrowing `aether_window` because a comment there explains which of the
--- two owns a widget's window procedure. A comment describing a boundary is not a
--- crossing of it, and counting one would make this script report the exact
--- situation it exists to find. Aether's `verify_test_inventory` learned the same
--- thing about a suite named in prose.
local function stripComments(body: string): string
	--- Block comments first, so a `//` inside one cannot survive as a line.
	body = string.gsub(body, "/%*.-%*/", " ")
	--- Then everything from `//` to the end of each line. `%c` rather than a
	--- newline literal so this file needs no escape that a shell can mangle.
	body = string.gsub(body, "//[^%c]*", "")
	return body
end

local NOT_OURS = { "target", "roblox_packages", ".pesde", "node_modules", ".git" }

local function walk(dir: string, out: { string })
	if not fs.isDir(dir) then
		return out
	end
	for _, entry in ipairs(fs.readDir(dir)) do
		local path = `{dir}/{entry}`
		if table.find(NOT_OURS, entry) then
			continue
		end
		if fs.isDir(path) then
			walk(path, out)
		else
			table.insert(out, path)
		end
	end
	return out
end

-- 1 - THE HOST IS RUST; THE GUEST BRINGS THE LUAU ----------------------------
--
-- `docs/mod_contract.md` states it: "Dew ships no Luau of its own". A host that
-- grew a Luau layer would be a second framework competing with the one its guests
-- already chose, and the mod contract would stop being a contract.

for _, path in ipairs(walk("host", {})) do
	if string.match(path, "%.luau?$") then
		fail(
			`{path}: Luau inside the host. Dew ships no Luau of its own -- the host is Rust `
				.. `and the guest brings the Luau. See docs/mod_contract.md.`
		)
	end
end

-- 2 - NOTHING NEW BORROWS FROM AETHER ----------------------------------------

local manifest = if fs.isFile("host/Cargo.toml") then fs.readFile("host/Cargo.toml") else ""
for line in string.gmatch(manifest, "[^\r\n]+") do
	local crate = string.match(line, "^(%S+)%s*=%s*{.*project%-aether%-ui/aether")
	if crate and not table.find(BORROWED_CRATES, crate) then
		fail(
			`host/Cargo.toml borrows {crate} from the Aether repository, which is not on the `
				.. `migration allowlist. Aether is a Luau framework; host code belongs here.`
		)
	end
end

--- A file reaching for a borrowed crate that has not declared it. The allowlist
--- shrinks as the crates move in and the imports become local.
for _, path in ipairs(walk("host/src", {})) do
	if not string.match(path, "%.rs$") then
		continue
	end
	local body = stripComments(fs.readFile(path))
	for _, crate in ipairs(BORROWED_CRATES) do
		if string.find(body, crate, 1, true) and not BORROWED_USE[path] then
			fail(
				`{path} uses {crate} and is not on the migration allowlist. Either it should not `
					.. `reach for Aether's Rust, or the allowlist needs the reason written down.`
			)
		end
	end
end

-- 3 - THE DATAMODEL IS DEW'S OWN ---------------------------------------------
--
-- The whole claim of ADR-001 is that a guest reaches a DataModel this host
-- implements. If the datamodel modules started deferring to Aether for anything
-- but the render IR, the standard would have two implementations inside one host.

for _, path in ipairs(walk("host/src/datamodel", {})) do
	if not string.match(path, "%.rs$") then
		continue
	end
	local body = stripComments(fs.readFile(path))
	for _, crate in ipairs(BORROWED_CRATES) do
		if string.find(body, crate, 1, true) and path ~= "host/src/datamodel/render.rs" then
			fail(
				`{path} reaches for {crate}. Only the render bridge may touch the display list; `
					.. `the DataModel itself is Dew's and answers to the reflection database.`
			)
		end
	end
end

-- 4 - THE DEBT, NAMED AND COUNTED --------------------------------------------

local outstanding: { string } = {}
for _, crate in ipairs(BORROWED_CRATES) do
	if string.find(manifest, crate, 1, true) then
		table.insert(outstanding, `{crate} (borrowed from the Aether repository)`)
	end
end
for path, why in pairs(BORROWED_USE) do
	if fs.isFile(path) then
		table.insert(outstanding, `{path} -> {why}`)
	end
end
table.sort(outstanding)

--- dew-cli does not exist, and Dew does not build without win32: `mod tray;` is
--- unconditional and `tray.rs` uses the `windows` crate unconditionally. Stated
--- rather than checked, because the check is "it compiles on Linux" and that is
--- CI's job on the day there is a Linux job to run it.
local mainRs = if fs.isFile("host/src/main.rs") then fs.readFile("host/src/main.rs") else ""
if string.find(mainRs, "\nmod tray;", 1, true) then
	table.insert(outstanding, "host/src/main.rs -> `mod tray;` is unconditional; no headless build")
end

print(`boundaries: {#outstanding} known violation(s) still to migrate`)
for _, entry in ipairs(outstanding) do
	print(`  DEBT  {entry}`)
end

print("")
if failures > 0 then
	print(`VERIFY BOUNDARIES: FAILED ({failures} problem(s))`)
	process.exit(1)
elseif #outstanding > 0 then
	print(`VERIFY BOUNDARIES: PASS (no new violations; {#outstanding} named for migration)`)
else
	print("VERIFY BOUNDARIES: PASS (Dew owns its host outright)")
end
return true

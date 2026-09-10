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
	was Dew's rendering stack living in the framework's repository, left there from
	when the zune experiment and the CLI needed it. None of it reached a Luau
	consumer, and after aether-cli retires Dew is its only dependent.

	IT MOVED, in milestone 2's first sprint: `crates/raster`, `crates/runtime` and
	`crates/window` are Dew's, and `host/Cargo.toml` names no git dependency on the
	Aether repository at all. What Dew still takes from Aether is LUAU, and it takes
	it the way it takes vide -- a pesde dependency pinned by commit in `pesde.toml`,
	installed into `roblox_packages/` as a guest package. Aether is a GUEST here,
	never a dependency of the host.

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

--- Dew's rendering stack. THESE ARE OURS NOW, and their names are the last thing
--- about them that is not.
---
--- They were three git dependencies on the Aether repository, plus four host
--- files listed as reaching for them. Both lists have left the debt below,
--- because `host/src/main.rs` using the window and the painter is not a boundary
--- crossing any more -- it is Dew's host calling Dew's crates, which is the
--- arrangement ADR-004 asked for.
---
--- WHAT THEY ARE STILL CHECKED FOR is section 3: only the render bridge may touch
--- the display list, so a datamodel module reaching for one of these would be a
--- second implementation of the standard growing inside one host.
---
--- `aether_raster`'s `gpu` feature gate survived the move, and section 5 checks
--- that it did rather than trusting it. It is what lets one crate serve a CPU
--- snapshot and a GPU swapchain, with the measurement recorded in its manifest,
--- and flattening it during a move would be a silent regression of exactly the
--- kind this script exists to catch -- one no test would notice, because both
--- halves still draw.
local RENDER_CRATES = {
	"dew_runtime",
	"dew_raster",
	"dew_window",
}

--- The display list decoding machinery now lives in dew_runtime as Dew's
--- private render IR driving Aether guests.
local DECODES_A_LUAU_DISPLAY_LIST: { string } = {}

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

--- `crates/` IS WALKED TOO, and it was not before, because before this sprint
--- there was nothing there. The rule is about Dew's host, and the host is now
--- four packages rather than one; checking only the original directory would have
--- left three quarters of it unchecked on the day it arrived.
local HOST_LUAU_ALLOWED: { string } = {}

for _, dir in ipairs({ "host", "crates" }) do
	for _, path in ipairs(walk(dir, {})) do
		--- A FIXTURE UNDER `tests/` IS A GUEST, AND THAT IS THE RULE RATHER THAN AN
		--- exception to it. `crates/runtime/tests/fixtures/app.luau` is an Aether
		--- application, written the way a mod is written, so that a Rust test can
		--- drive one. Counting it would say the host ships Luau because it has a
		--- test that loads some -- which is the opposite of what happens.
		if string.find(path, "/tests/", 1, true) then
			continue
		end
		if string.match(path, "%.luau?$") and not table.find(HOST_LUAU_ALLOWED, path) then
			fail(
				`{path}: Luau inside the host. Dew ships no Luau of its own -- the host is Rust `
					.. `and the guest brings the Luau. See docs/mod_contract.md.`
			)
		end
	end
end

-- 2 - NO RUST COMES FROM AETHER ----------------------------------------------
--
-- THE ALLOWLIST HERE IS GONE RATHER THAN SHORTER, which is what "the crates
-- moved" means when it is a check instead of a sentence. Any Cargo dependency on
-- that repository now fails on the commit that adds it, whatever it is called.

local manifest = if fs.isFile("host/Cargo.toml") then fs.readFile("host/Cargo.toml") else ""
for line in string.gmatch(manifest, "[^\r\n]+") do
	local crate = string.match(line, "^(%S+)%s*=%s*{.*project%-aether%-ui/aether")
	if crate then
		fail(
			`host/Cargo.toml takes {crate} from the Aether repository. Aether is a LUAU `
				.. `framework and a guest here; there is no Rust in that repository to depend `
				.. `on. Dew's rendering stack is in crates/.`
		)
	end
end

--- And the same of the crates that moved: a dependency reaching back out to
--- Aether would be the git pin again, wearing a different hat.
for _, crate in ipairs(RENDER_CRATES) do
	local path = `crates/{string.gsub(crate, "^dew_", "")}/Cargo.toml`
	if fs.isFile(path) and string.find(fs.readFile(path), "project-aether-ui/aether", 1, true) then
		fail(`{path} reaches back into the Aether repository. It lives here now.`)
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
	for _, crate in ipairs(RENDER_CRATES) do
		if string.find(body, crate, 1, true) and path ~= "host/src/datamodel/render.rs" then
			fail(
				`{path} reaches for {crate}. Only the render bridge may touch the display list; `
					.. `the DataModel itself is Dew's and answers to the reflection database.`
			)
		end
	end
end

-- 4 - THE `gpu` FEATURE GATE SURVIVED THE MOVE -------------------------------
--
-- CHECKED, NOT TRUSTED, AND THIS IS THE ONE THING IN THIS SPRINT NO TEST COULD
-- HAVE CAUGHT. `aether_raster` serves a CPU snapshot and a GPU swapchain from one
-- crate because `vello_hybrid` and `wgpu` are optional and OFF by default -- a
-- Roblox author running visual diffs in CI never compiles wgpu, and the
-- measurement that decided it is recorded in the manifest. Flattening the gate
-- while relocating the file would leave every pixel identical and every test
-- green, and would cost everyone a wgpu build. ADR-004 names it as consequence
-- five for exactly that reason.

local rasterManifest = if fs.isFile("crates/raster/Cargo.toml")
	then fs.readFile("crates/raster/Cargo.toml")
	else ""
if rasterManifest ~= "" then
	if not string.find(rasterManifest, "\ngpu = %[") then
		fail(
			"crates/raster/Cargo.toml has no `gpu` feature. It is what lets one crate serve a "
				.. "CPU snapshot and a GPU swapchain; flattening it is a silent regression."
		)
	end
	--- OFF BY DEFAULT is the half that costs something. A `gpu` feature listed
	--- under `default` is the same build for everybody with an extra name.
	local default = string.match(rasterManifest, "\ndefault%s*=%s*(%b[])")
	if default and string.find(default, "gpu", 1, true) then
		fail(
			"crates/raster/Cargo.toml has `gpu` in its default features. Off by default is the "
				.. "point: a headless snapshot must not compile wgpu."
		)
	end
	for _, optional in ipairs({ "vello_hybrid", "wgpu" }) do
		local line = string.match(rasterManifest, `\n{optional}%s*=%s*[^\r\n]+`) or ""
		if line ~= "" and not string.find(line, "optional = true", 1, true) then
			fail(
				`crates/raster/Cargo.toml declares {optional} without \`optional = true\`. That `
					.. `compiles it for every consumer whether or not \`gpu\` is on.`
			)
		end
	end
end

-- 5 - THE DEBT, NAMED AND COUNTED --------------------------------------------

local outstanding: { string } = {}

--- THE NAMES: three crates in Dew's own tree renamed from aether_* to dew_*.
for _, oldName in ipairs({ "aether_runtime", "aether_raster", "aether_window" }) do
	local dir = `crates/{string.gsub(oldName, "^aether_", "")}/Cargo.toml`
	if fs.isFile(dir) and string.find(fs.readFile(dir), `name = "{oldName}"`, 1, true) then
		table.insert(outstanding, `{dir} -> still named {oldName}; Dew's crate, Aether's name`)
	end
end

for _, path in ipairs(HOST_LUAU_ALLOWED) do
	if fs.isFile(path) then
		table.insert(outstanding, `{path} -> Luau in the host; a zune-era check of a dead path`)
	end
end

--- The decoding half, which retires with `LuauDataModel.luau` in sprint 6.
local retiring: { string } = {}
for _, path in ipairs(DECODES_A_LUAU_DISPLAY_LIST) do
	if fs.isFile(path) then
		table.insert(retiring, string.match(path, "([^/]+)$") :: string)
	end
end
if #retiring > 0 then
	table.insert(
		outstanding,
		`crates/runtime -> {table.concat(retiring, ", ")} decode what Live.luau emits; `
			.. `they retire with LuauDataModel.luau`
	)
end

--- THE HOST DECLARES NO FRAMEWORK, and this is what keeps that true.
---
--- Milestone 6 emptied `[dependencies]`: a mod declares Aether and vide in its
--- own manifest and reaches them through the redirect pesde writes beside it, so
--- nothing Dew ships needs a framework. Both stayed on as DEV dependencies,
--- because Dew tests against them and `dew test` reads suites out of a checkout.
---
--- The pressure this resists is an application in the monorepo -- a demo, a
--- sample, a scratch app -- finding it convenient to add its dependency at the
--- root rather than carry its own manifest. That is one line, it works
--- immediately, and it silently re-couples the host to a guest framework.
--- ADR-008 puts the framework demo in this repository on exactly that condition.
local manifest = fs.readFile("pesde.toml")
local deps = string.match(manifest, "\n%[dependencies%]\n(.-)\n%[")
if deps then
	for line in string.gmatch(deps, "[^\n]+") do
		local trimmed = string.gsub(line, "^%s+", "")
		if trimmed ~= "" and string.sub(trimmed, 1, 1) ~= "#" then
			local name = string.match(trimmed, "^([%w_%-]+)%s*=") or trimmed
			fail(
				`pesde.toml declares '{name}' under [dependencies]. The host ships no `
					.. "framework: an application that needs one carries its own manifest, the "
					.. "way mods/*/pesde.toml do. Move it to [dev_dependencies] if Dew's own "
					.. "tests need it."
			)
		end
	end
end

table.sort(outstanding)

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

--!strict
--[[
	verify_ascii -- plain ASCII, in every file this repository writes.

	    lune run scripts/verify_ascii.luau

	WHY
	`CONTRIBUTING.md` says it, `docs/contributing/guidelines.md` says it, and
	`docs/contributing/guides/writing.md` gives it a section: plain ASCII, in
	commits, pull requests and documentation alike. NOTHING CHECKED IT, and the
	rule has been losing for as long as it has existed -- 2758 characters across
	60 files on this script's first run, in the host's own source and in what Dew
	prints to a terminal.

	It is the same argument `verify_boundaries.luau` makes in its own header: a
	rule with nothing checking it is a rule nobody follows. Six sprint records
	drifted over four days and were normalised by hand, which is the fix that does
	not stay fixed. This is the fix that does.

	THE BYTE-ORDER MARK IS THE ONE THAT BITES. `scripts/commitlint.luau` carried
	one, so it did not parse, so commits landed for months using scopes it would
	have rejected -- a validator reporting nothing while doing nothing. On this
	script's first run `.gitignore` and `VERSION` were both carrying one,
	invisible to every check in the repository and to every diff. Both are gone in
	the commit that added this file. Nothing reads `VERSION` today; the day
	something does, `parseSemVer` gets three bytes it cannot match and the version
	is "not valid SemVer" for a reason that does not appear on screen.

	THE CENSUS BELOW IS THE MIGRATION, exactly as the allowlist is in
	`verify_boundaries.luau`. A file may carry no more than it carried on the day
	this script landed, so every NEW violation fails on the commit that adds it,
	while the existing population is named, counted, and shrinks under its own
	commits. A file absent from the census gets no allowance at all --
	`.artifacts/project/` is at zero for that reason, the sprint records having
	been cleaned by hand the day before.

	Emptying it is what finishing looks like from here.

	`.artifacts/` IS WALKED, and git does not track it. That is deliberate: the
	drift this script exists to catch happened entirely inside it, and a check
	scoped to tracked files would have passed on every one of those six records.
	It is simply absent in CI, where the check earns its place on the rest.
]]

local fs = require("@lune/fs")
local process = require("@lune/process")

--- Generated or vendored. `Cargo.lock` is tracked and reproducible, but it is
--- cargo's to write, and a dependency with a non-ASCII name is not ours to fix.
local NOT_OURS = {
	"target",
	"roblox_packages",
	"luau_packages",
	".pesde",
	"node_modules",
	"dist",
	".git",
	"Cargo.lock",
	"pesde.lock",
}

--- BY EXTENSION, NOT BY SNIFFING. A PNG is full of high bytes and is not a
--- violation; guessing which files are text from their contents would make this
--- script's answer depend on what a file happens to contain. Extensionless names
--- that are text are listed outright.
local TEXT_EXTENSIONS = { "md", "luau", "rs", "toml", "json", "yml", "yaml", "txt", "gitignore" }
local TEXT_NAMES = { "VERSION", "LICENSE", "CONTRIBUTING", "README" }

--- WHAT WAS THERE ON 2026-09-04, per file. Lower a number when you clean a file;
--- never raise one. An entry whose file is gone or already clean is reported as
--- slack to remove, because an allowance nobody has earned is a hole.
local CENSUS: { [string]: number } = {
	[".artifacts/branching_strategy_and_scope_discipline.md"] = 178,
	[".artifacts/drafts/CONTRIBUTING.md"] = 2,
	[".artifacts/drafts/README_PUBLIC_LAUNCH.md"] = 16,
	[".artifacts/drafts/docs/contributing/branching.md"] = 10,
	[".artifacts/drafts/docs/contributing/guidelines.md"] = 6,
	[".artifacts/drafts/docs/internals/architecture/mod_ecosystem.md"] = 14,
	[".artifacts/drafts/docs/internals/architecture/stack.md"] = 7,
	[".artifacts/drafts/docs/internals/hot_reload/watcher.md"] = 26,
	[".artifacts/drafts/docs/internals/ipc/protocol.md"] = 52,
	[".artifacts/drafts/docs/internals/runtime/luau_vm.md"] = 66,
	[".artifacts/drafts/docs/mods/mod_authoring.md"] = 19,
	[".artifacts/early_versioning_and_release_policy.md"] = 3,
	[".artifacts/repo_hygiene_and_release_strategy.md"] = 10,
	[".artifacts/vision/aether_native_architecture_pivot.md"] = 395,
	[".artifacts/vision/vision_chat_dump.md"] = 8,
	["Cargo.toml"] = 3,
	["README.md"] = 2,
	["crates/raster/src/bitmap.rs"] = 2,
	["crates/raster/src/canvas.rs"] = 8,
	["crates/raster/src/hybrid_probe.rs"] = 1,
	["crates/raster/src/lib.rs"] = 20,
	["crates/raster/verify.luau"] = 1,
	["crates/runtime/README.md"] = 1,
	["crates/runtime/src/driver.rs"] = 3,
	["crates/runtime/src/font.rs"] = 2,
	["crates/runtime/src/frame.rs"] = 10,
	["crates/runtime/src/lib.rs"] = 28,
	["crates/runtime/src/modules.rs"] = 4,
	["crates/runtime/src/painter.rs"] = 11,
	["crates/runtime/src/raster.rs"] = 14,
	["crates/runtime/src/requirer.rs"] = 4,
	["crates/runtime/src/session.rs"] = 6,
	["crates/runtime/src/vm.rs"] = 5,
	["crates/runtime/tests/example.rs"] = 8,
	["crates/runtime/tests/fixtures/app.luau"] = 1,
	["crates/runtime/tests/fixtures/pressable.luau"] = 1,
	["crates/runtime/tests/frame_contract.rs"] = 2,
	["crates/runtime/tests/interaction.rs"] = 3,
	["crates/runtime/tests/parity.rs"] = 8,
	["crates/runtime/tests/render.rs"] = 12,
	["crates/window/src/lib.rs"] = 8,
	["crates/window/src/win32.rs"] = 9,
	["docs/mod_contract.md"] = 20,
	["host/Cargo.toml"] = 1,
	["host/src/bin/datamodel_surface.rs"] = 67,
	["host/src/capabilities.rs"] = 2,
	["host/src/datamodel/input.rs"] = 472,
	["host/src/datamodel/members.rs"] = 426,
	["host/src/datamodel/mod.rs"] = 172,
	["host/src/datamodel/render.rs"] = 67,
	["host/src/datamodel/signal.rs"] = 423,
	["host/src/main.rs"] = 29,
	["host/src/manifest.rs"] = 3,
	["host/src/mods.rs"] = 38,
	["host/src/surface.rs"] = 3,
	["host/src/tray.rs"] = 4,
	["mods/calculator/calculator.luau"] = 6,
	["mods/nameplate/nameplate.luau"] = 22,
	["mods/timetracker/timetracker.luau"] = 7,
	["mods/workspace/workspace.luau"] = 7,
}

local function isText(path: string): boolean
	local name = string.match(path, "([^/]+)$") or path
	if table.find(TEXT_NAMES, name) then
		return true
	end
	local ext = string.match(name, "%.([^.]+)$")
	return ext ~= nil and table.find(TEXT_EXTENSIONS, ext) ~= nil
end

local function walk(dir: string, out: { string })
	if not fs.isDir(dir) then
		return out
	end
	for _, entry in ipairs(fs.readDir(dir)) do
		if table.find(NOT_OURS, entry) then
			continue
		end
		local path = `{dir}/{entry}`
		if fs.isDir(path) then
			walk(path, out)
		elseif isText(path) then
			table.insert(out, path)
		end
	end
	return out
end

--- NAME THE CHARACTER AND GIVE THE REPLACEMENT. "non-ASCII byte at line 12" sends
--- someone hunting with a hex editor for a glyph that looks exactly like the one
--- beside it. Every entry here is a character this repository actually carries.
local KNOWN: { [number]: string } = {
	[0xFEFF] = "byte-order mark -- delete it; this is the one that stops a file parsing",
	[0x2014] = "em dash -- write two hyphens",
	[0x2013] = "en dash -- write one hyphen",
	[0x2018] = "left single quote -- write a plain apostrophe",
	[0x2019] = "right single quote -- write a plain apostrophe",
	[0x201C] = "left double quote -- write a plain double quote",
	[0x201D] = "right double quote -- write a plain double quote",
	[0x00B5] = "micro sign -- write `us`",
	[0x03BC] = "greek mu -- write `us`",
	[0x00B7] = "middle dot -- write a hyphen or an asterisk",
	[0x2022] = "bullet -- write a hyphen",
	[0x2026] = "ellipsis -- write three dots",
	[0x00A0] = "non-breaking space -- write a plain space",
	[0x2192] = "rightwards arrow -- write `->`",
	[0x2500] = "box drawing -- rule a heading with hyphens, as verify_boundaries.luau does",
	[0x2502] = "box drawing -- draw a diagram with a pipe",
	[0x2514] = "box drawing -- draw a diagram in ASCII",
	[0x251C] = "box drawing -- draw a diagram in ASCII",
	[0x250C] = "box drawing -- draw a diagram in ASCII",
	[0x2510] = "box drawing -- draw a diagram in ASCII",
	[0x2518] = "box drawing -- draw a diagram in ASCII",
	[0x2534] = "box drawing -- draw a diagram in ASCII",
	[0x25CF] = "filled circle -- write an asterisk",
	[0x25BC] = "down triangle -- write `v`",
	[0x25B2] = "up triangle -- write a caret",
	[0xFE0F] = "emoji variation selector -- delete it with the emoji it follows",
	[0x2713] = "check mark -- write `[x]` or the word",
	[0x2705] = "check mark emoji -- write the word",
}

--- One report per character, but only the first few per file: 472 box-drawing
--- rules in one file's comment headers is one problem, and printing it 472 times
--- buries the file that has a single smart quote in it.
local PER_FILE_REPORTS = 4

local files = walk(".", {})
table.sort(files)

local counts: { [string]: number } = {}
local failures = 0
local total = 0

for _, path in ipairs(files) do
	--- `./` is how `walk` names things and is not how anyone writes a path.
	local clean = string.gsub(path, "^%./", "")
	local body = fs.readFile(path)
	--- Cheap first pass. Only files carrying a high byte pay for the walk below.
	if not string.find(body, "[\128-\255]") then
		continue
	end

	local found = 0
	local reports: { string } = {}
	local line = 0
	for text in string.gmatch(body .. "\n", "([^\n]*)\n") do
		line += 1
		local column = string.find(text, "[\128-\255]")
		while column do
			--- `utf8.codepoint` errors on bytes that are not valid UTF-8, which is
			--- itself worth reporting rather than crashing on.
			local ok, code = pcall(utf8.codepoint, text, column)
			local described = if ok and KNOWN[code :: number]
				then KNOWN[code :: number]
				else if ok
					then `U+{string.format("%04X", code :: number)} -- write it in ASCII`
					else "not valid UTF-8"
			found += 1
			if #reports < PER_FILE_REPORTS then
				table.insert(reports, `{clean}:{line}:{column}  {described}`)
			end
			--- Past this CHARACTER, not past this byte: a multi-byte sequence would
			--- otherwise be counted once per continuation byte.
			local nextStart = if ok then (utf8.offset(text, 2, column) or #text + 1) else column + 1
			column = string.find(text, "[\128-\255]", nextStart)
		end
	end

	counts[clean] = found
	total += found

	local allowed = CENSUS[clean] or 0
	if found > allowed then
		failures += 1
		print(`FAIL  {clean}: {found} non-ASCII character(s), {allowed} allowed`)
		for _, report in ipairs(reports) do
			print(`        {report}`)
		end
		if found > #reports then
			print(`        ... and {found - #reports} more`)
		end
	end
end

--- SLACK IS A HOLE. An allowance larger than the file needs lets the next
--- violation in for free, and this repository has already found five checks that
--- reported success while doing nothing.
local slack = 0
for path, allowed in CENSUS do
	local found = counts[path]
	if found == nil then
		print(`SLACK {path}: clean or gone; delete its census entry ({allowed} allowed)`)
		slack += allowed
	elseif found < allowed then
		print(`SLACK {path}: {found} left of {allowed} allowed; lower it to {found}`)
		slack += allowed - found
	end
end

print("")
print(`ascii: {total} character(s) still to migrate, in {#files} file(s) checked`)
if failures > 0 then
	print(`VERIFY ASCII: FAILED ({failures} file(s) over their allowance)`)
	process.exit(1)
elseif slack > 0 then
	print(`VERIFY ASCII: FAILED (census is {slack} character(s) stale; tighten it)`)
	process.exit(1)
elseif total > 0 then
	print(`VERIFY ASCII: PASS (no new violations; {total} named for migration)`)
else
	print("VERIFY ASCII: PASS (plain ASCII throughout)")
end
return true

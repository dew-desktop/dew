--!strict
--[[
	setup -- everything this repository expects of a working copy, checked once.

	    lune run scripts/setup.luau

	Idempotent, and it reports rather than assumes: run it again any time to see
	what is in place. It changes exactly one thing, the hooks path, and only when
	nothing else has claimed it.
]]

local fs = require("@lune/fs")
local process = require("@lune/process")

local WINDOWS = process.os == "windows"

local function has(tool: string): boolean
	local probe = if WINDOWS then { "-Command", `Get-Command {tool} -ErrorAction SilentlyContinue` } else { "-c", `command -v {tool}` }
	local shell = if WINDOWS then "powershell" else "sh"
	local result = process.exec(shell, probe)
	return result.ok and result.stdout ~= ""
end

local function config(scope: string, key: string): string?
	local result = process.exec("git", { "config", scope, key })
	if not result.ok then
		return nil
	end
	local value = string.match(result.stdout, "^%s*(.-)%s*$")
	return if value == "" then nil else value
end

local notes: { string } = {}
local blocked = false

print("dew: checking this working copy")
print("")

--- THE TOOLS THIS REPOSITORY RUNS ON.
for _, tool in ipairs({ "cargo", "lune", "pesde" }) do
	if has(tool) then
		print(`  ok       {tool}`)
	else
		print(`  MISSING  {tool}`)
		blocked = true
		if tool == "lune" then
			table.insert(notes, "lune comes from rokit: `rokit install` at the repository root.")
		elseif tool == "pesde" then
			table.insert(notes, "pesde is pinned in .github/workflows/ci.yml rather than rokit.toml; see the note there.")
		else
			table.insert(notes, "cargo comes from rustup, pinned by rust-toolchain.toml.")
		end
	end
end

--- THE GATES, WHICH RUN BEFORE A PUSH RATHER THAN ON A RUNNER.
---
--- A GLOBAL HOOKS PATH IS LEFT ALONE. `core.hooksPath` holds one value and a
--- local one wins, so setting it here would silently disable whatever a
--- developer has set up across their machine. The hook in this repository is
--- designed to be called by one of those, so delegation is the arrangement that
--- works either way.
print("")
local localHooks = config("--local", "core.hooksPath")
local globalHooks = config("--global", "core.hooksPath")

if localHooks == ".githooks" then
	print("  ok       pre-push gates (this repository)")
elseif globalHooks then
	print(`  ok       pre-push gates (from {globalHooks})`)
	table.insert(
		notes,
		`A global hooks path is set, so this left it alone. Make sure it runs .githooks/pre-push, or this repository's gates will not run.`
	)
else
	local result = process.exec("git", { "config", "core.hooksPath", ".githooks" })
	if result.ok then
		print("  set      pre-push gates (this repository)")
	else
		print("  FAILED   could not set core.hooksPath")
		blocked = true
	end
end

--- WHAT DEW'S OWN TOOLING NEEDS.
---
--- NOT A DEPENDENCY OF DEW. `[dependencies]` in the root manifest is empty and a
--- gate keeps it that way: nothing Dew ships needs a framework. The pins are
--- `[dev_dependencies]`, because the conformance runner and the coverage tool
--- need a framework to read cases and measure against, and `framework-coverage`
--- exits rather than reporting a zero when it cannot find one.
print("")
if fs.isDir("roblox_packages/.pesde") then
	print("  ok       the framework Dew tests against")
else
	print("  MISSING  the framework Dew tests against")
	blocked = true
	table.insert(notes, "`pesde install` at the repository root. Without it the conformance runner and framework-coverage have nothing to read.")
end

--- WHAT AN EXAMPLE NEEDS BEFORE IT RUNS.
print("")
local missingPackages: { string } = {}
for _, group in ipairs({ "examples/aether", "examples/widgets", "examples/host" }) do
	if not fs.isDir(group) then
		continue
	end
	for _, entry in ipairs(fs.readDir(group)) do
		local dir = `{group}/{entry}`
		if fs.isFile(`{dir}/pesde.toml`) and not fs.isFile(`{dir}/roblox_packages/aether.luau`) then
			table.insert(missingPackages, dir)
		end
	end
end

if #missingPackages == 0 then
	print("  ok       every example has its own packages")
else
	print(`  MISSING  packages for {#missingPackages} example(s)`)
	for _, dir in ipairs(missingPackages) do
		print(`             {dir}`)
	end
	table.insert(notes, "Each example brings its own framework: `pesde install` inside the directory.")
end

print("")
if #notes > 0 then
	for _, note in ipairs(notes) do
		print(`  note: {note}`)
	end
	print("")
end

if blocked then
	print("dew: something above is missing. The gates will not all run until it is there.")
	process.exit(1)
end

print("dew: ready. Read docs/contributing/guidelines.md next, and run these any time:")
print("")
print("    lune run scripts/smoke.luau               every command the CLI promises")
print("    lune run scripts/commitlint_check.luau    commit subjects, on the range you are pushing")
print("    cargo test --manifest-path host/Cargo.toml")

--!strict
--[[
	smoke -- every command Dew's CLI promises, run against a real binary.

	    lune run scripts/smoke.luau            build, then run everything
	    lune run scripts/smoke.luau --no-build  use the binary already built

	WHY THIS IS A SCRIPT AND NOT A CI STEP

	These eleven invocations lived only in `.github/workflows/ci.yml`. Nothing but
	CI ran them, so nothing but CI could notice when they went stale, and on
	2026-09-13 two of them had:

	    `check applets/*/`   relied on the shell expanding the glob, which pwsh
	                         does not do for a native executable, so `dew` went
	                         looking for a directory named `*`.
	    `snapshot --script`  passed its output path positionally, which became a
	                         second target when applets started being named by path.

	Both were found by a pull request rather than by the person who made the
	change, and each cost a full round trip to learn something a second of local
	work would have said. A sequence that only one machine can run is a sequence
	that rots between the times that machine runs it.

	SO CI CALLS THIS FILE. Not a copy of it, this file, which is the only
	arrangement where local and CI cannot disagree.

	NO SHELL GLOBS ANYWHERE. Discovery happens here, in Luau, because the bug
	above was the shell being asked to do it on two platforms that disagree.
]]

local fs = require("@lune/fs")
local process = require("@lune/process")

local WINDOWS = process.os == "windows"
local EXE = if WINDOWS then "target/debug/dew.exe" else "target/debug/dew"

--- Somewhere to write PNGs that is not the repository.
local TEMP = if WINDOWS
	then (process.env.TEMP or process.env.TMP or ".")
	else (process.env.TMPDIR or "/tmp")

local failures: { string } = {}
local ran = 0

--- Run `dew` with these arguments and say what happened.
---
--- THE BINARY DIRECTLY, not `cargo run`. Eleven `cargo run` invocations each pay
--- for a freshness check; one build followed by eleven executions is the same
--- coverage in a fraction of the time, and the time is the whole point.
local function dew(args: { string }): (boolean, string)
	local result = process.exec(EXE, args)
	return result.ok, result.stdout .. result.stderr
end

--- A step that has to succeed.
local function ok(name: string, args: { string }, expect: string?)
	ran += 1
	local succeeded, output = dew(args)
	if not succeeded then
		table.insert(failures, `{name}: dew exited non-zero\n    dew {table.concat(args, " ")}\n{output}`)
		print(`  FAIL  {name}`)
		return
	end
	if expect and not string.find(output, expect, 1, true) then
		table.insert(
			failures,
			`{name}: output did not contain {string.format("%q", expect)}\n{output}`
		)
		print(`  FAIL  {name}`)
		return
	end
	
	--- AN `-o` PATH IS CHECKED FOR CONTENT, not merely for existence. A painter
	--- that writes a zero-byte PNG exits zero and leaves a file, so "the command
	--- worked" and "something was drawn" are different claims, and only the second
	--- is worth making.
	local wrote = table.find(args, "-o")
	if wrote then
		local path = args[wrote + 1]
		if not path or not fs.isFile(path) then
			table.insert(failures, `{name}: wrote no file at {tostring(path)}`)
			print(`  FAIL  {name}`)
			return
		end
		if #fs.readFile(path) == 0 then
			table.insert(failures, `{name}: {path} is empty`)
			print(`  FAIL  {name}`)
			return
		end
	end
	print(`  ok    {name}`)
end

--- A step that has to be REFUSED, and refused for the stated reason.
---
--- A command that fails for the wrong reason passes a test that only checks the
--- exit code, which is how a typo in an error message goes unnoticed for months.
local function refused(name: string, args: { string }, expect: string)
	ran += 1
	local succeeded, output = dew(args)
	if succeeded then
		table.insert(failures, `{name}: expected dew to refuse this and it did not\n{output}`)
		print(`  FAIL  {name}`)
		return
	end
	if not string.find(output, expect, 1, true) then
		table.insert(
			failures,
			`{name}: refused for the wrong reason, wanted {string.format("%q", expect)}\n{output}`
		)
		print(`  FAIL  {name}`)
		return
	end
	print(`  ok    {name}`)
end

--- Every applet directory under a parent, found here rather than by a shell.
local function appletsIn(parent: string): { string }
	local found: { string } = {}
	if not fs.isDir(parent) then
		return found
	end
	for _, entry in ipairs(fs.readDir(parent)) do
		local path = `{parent}/{entry}`
		if fs.isDir(path) and fs.isFile(`{path}/dew.toml`) then
			table.insert(found, path)
		end
	end
	table.sort(found)
	return found
end

local skipBuild = false
for _, arg in ipairs(process.args) do
	if arg == "--no-build" then
		skipBuild = true
	end
end

if not skipBuild then
	print("building dew")
	local built = process.exec("cargo", {
		"build",
		"--quiet",
		"--manifest-path",
		"host/Cargo.toml",
		"--bin",
		"dew",
	}, { stdio = "forward" })
	if not built.ok then
		print("smoke: the build failed, so nothing below would mean anything")
		process.exit(1)
	end
end

if not fs.isFile(EXE) then
	print(`smoke: no binary at {EXE}; run without --no-build`)
	process.exit(1)
end

--- EVERY EXAMPLE, not the first one. A broken applet is skipped at runtime with
--- a message and the host carries on, which is right when a user has ten
--- installed and wrong here, where the skip is the thing being tested for.
---
--- THE GROUPS ARE NAMED, not discovered. A walk would pick up whatever landed
--- under `examples/` next, and a suite that silently grows is one nobody reads
--- the output of.
local targets: { string } = {}
for _, group in ipairs({ "examples/aether", "examples/widgets", "examples/host" }) do
	for _, found in ipairs(appletsIn(group)) do
		table.insert(targets, found)
	end
end

if #targets == 0 then
	print("smoke: found no applets, which cannot be right")
	process.exit(1)
end

print("the build identifier")
ok("--version prints the build identifier", { "--version" }, "+g")
ok("-V is the same command", { "-V" }, "+g")

print(`checking {#targets} applet(s)`)
ok("check every applet", (function()
	local args = { "check" }
	for _, target in ipairs(targets) do
		table.insert(args, target)
	end
	return args
end)())

print("mounting each applet")
for _, target in ipairs(targets) do
	local name = string.match(target, "[^/]+$") or target
	ok(`snapshot {target}`, { "snapshot", target, "-o", `{TEMP}/smoke_{name}.png` })
end

print("the standalone script path")
ok("snapshot a bare script", {
	"snapshot",
	"--script",
	"examples/host/standalone/app.luau",
	"--size",
	"360x220",
	"-o",
	`{TEMP}/smoke_standalone.png`,
})

print("selecting an applet by path")
ok(
	"snapshot names the applet it rendered",
	{ "snapshot", "examples/widgets/nameplate", "-o", `{TEMP}/smoke_nameplate.png` },
	"for nameplate"
)

print("scaffolding")
local SCAFFOLD = "examples/host/smoke_scaffold"
if fs.isDir(SCAFFOLD) then
	fs.removeDir(SCAFFOLD)
end
ok("init scaffolds an applet", { "init", SCAFFOLD })
ok("the scaffold passes check", { "check", SCAFFOLD })
ok("the scaffold renders", { "snapshot", SCAFFOLD, "-o", `{TEMP}/smoke_scaffold.png` })
if fs.isDir(SCAFFOLD) then
	fs.removeDir(SCAFFOLD)
end
refused(
	"an unknown surface is refused",
	{ "init", "examples/host/smoke_bad", "--surface", "windwo" },
	"unknown surface"
)
if fs.isDir("examples/host/smoke_bad") then
	fs.removeDir("examples/host/smoke_bad")
end

--- THE WINDOWED COMMANDS, WHERE THEY CANNOT RUN. Off Windows these drive a window
--- loop that has no window, and saying so beats hanging.
if not WINDOWS then
	print("the windowed commands off Windows")
	for _, command in ipairs({ "run", "--stats", "--bench" }) do
		refused(`{command} is refused off Windows`, { command }, "is Windows-only")
	end
end

print("")
if #failures > 0 then
	for _, failure in ipairs(failures) do
		print(failure)
		print("")
	end
	print(`smoke: {#failures} of {ran} step(s) failed`)
	process.exit(1)
end

print(`SMOKE: PASS ({ran} step(s))`)

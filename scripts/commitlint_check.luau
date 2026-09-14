--!strict
--[[
	Validate commit messages against scripts/commitlint.luau.

	    lune run scripts/commitlint_check.luau                 # HEAD
	    lune run scripts/commitlint_check.luau origin/main..   # a range

	The validator was a module nothing called: no CI, no hook, and no entry
	point. It also carried a byte-order mark, so it did not parse. Both are why
	commits landed for months using scopes it would have rejected.

	FIVE HISTORICAL COMMITS WILL ALWAYS FAIL, scoped `ui` and `dev` for
	directories that no longer exist. That is the validator working rather than
	breaking, and it is why the default is HEAD alone: check what you are about
	to add, not what is already written.
]]

local process = require("@lune/process")
local commitlint = require("./commitlint")

--- THE WHOLE MESSAGE, NOT THE SUBJECT. This read `--format=%s` and so checked
--- subjects alone, which meant the body rules could not fire: a tool attribution
--- trailer and a reference to the gitignored plan both sat in bodies this script
--- reported as valid.
---
--- A NUL SEPARATES COMMITS, because a body contains blank lines and a separator
--- made of them cannot say where one message ends.
local SEPARATOR = string.char(0)

local range = process.args[1]
local args = if range
	then { "log", "--format=%B%x00", range }
	else { "log", "--format=%B%x00", "-1" }

local result = process.exec("git", args)
if not result.ok then
	--- REPORT WHAT GIT SAID. "could not read the log" sent me looking at the
	--- range syntax when the answer was in stderr the whole time.
	print("commitlint: git failed")
	print(result.stderr)
	process.exit(1)
end

local failed = 0
local checked = 0

for _, message in ipairs(string.split(result.stdout, SEPARATOR)) do
	local trimmed = string.match(message, "^%s*(.-)%s*$") or ""
	if trimmed == "" then
		continue
	end
	local subject = string.match(trimmed, "^%C+") or trimmed

	--- Merge commits are exempt. Their subject is written by the forge and is
	--- "Title (#1)" or "Merge pull request #...", which no conventional-commit rule describes.
	if string.match(subject, "^Merge ") or string.match(subject, "%(#%d+%)$") then
		continue
	end
	checked += 1
	local verdict = commitlint.validate(trimmed)
	if not verdict.valid then
		failed += 1
		print(`REJECTED  {subject}`)
		for _, err in ipairs(verdict.errors) do
			print(`          {err}`)
		end
	end
end

print(`\ncommitlint: {checked - failed} of {checked} valid`)
if failed > 0 then
	process.exit(1)
end

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

local range = process.args[1]
local args = if range
	then { "log", "--format=%s", range }
	else { "log", "--format=%s", "-1" }

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

for line in string.gmatch(result.stdout, "[^\r\n]+") do
	--- Merge commits are exempt. Their subject is written by the forge and is
	--- "Title (#1)" or "Merge pull request #...", which no conventional-commit rule describes.
	if string.match(line, "^Merge ") or string.match(line, "%(#%d+%)$") then
		continue
	end
	checked += 1
	local verdict = commitlint.validate(line)
	if not verdict.valid then
		failed += 1
		print(`REJECTED  {line}`)
		for _, err in ipairs(verdict.errors) do
			print(`          {err}`)
		end
	end
end

print(`\ncommitlint: {checked - failed} of {checked} valid`)
if failed > 0 then
	process.exit(1)
end

--!strict
--[[
	pr_check -- the same writing rules, applied to a pull request.

	    PR_TITLE="..." PR_BODY="..." lune run scripts/pr_check.luau

	A PULL REQUEST IS THE MOST READ THING THIS REPOSITORY PRODUCES and the least
	checked. Commit messages had a linter and pull request bodies had a style
	guide, so the generated-by footer the guide forbids shipped in one anyway.

	Reads the environment rather than arguments, because a body is multi-line and
	contains every character a shell treats specially.
]]

local process = require("@lune/process")
local style = require("./house_style")

local title = process.env.PR_TITLE or ""
local body = process.env.PR_BODY or ""

if title == "" then
	print("pr_check: no PR_TITLE in the environment, nothing to check")
	process.exit(0)
end

local problems: { string } = {}

--- The title is a commit subject and obeys the same rule.
local summary = string.match(title, "^[%w_]+%([%w%-_/]+%):%s+(.+)$")
	or string.match(title, "^[%w_]+:%s+(.+)$")
if summary then
	local problem = style.subjectProblem(summary)
	if problem then
		table.insert(problems, `title: {problem}`)
	end
end

for _, finding in ipairs(style.findForbidden(`{title}\n{body}`)) do
	if finding.rule == "attribution" then
		table.insert(
			problems,
			`Remove {finding.detail}. A pull request is authored by the person opening it, and a footer naming a tool invites a reader to assume the prose above it was not thought through.`
		)
	else
		table.insert(
			problems,
			`Remove {finding.detail}. It names something the reader cannot open, and it dates the moment the plan moves on; say what the work is for instead.`
		)
	end
end

if #problems > 0 then
	print("pr_check: the pull request needs an edit before it can land")
	print("")
	for _, problem in ipairs(problems) do
		print(`  {problem}`)
	end
	print("")
	print("See docs/contributing/guides/writing.md.")
	process.exit(1)
end

print("PR CHECK: PASS")

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

--- HEADINGS ARE FOR NAVIGATING, and a body a reader gets through in one pass has
--- nothing to navigate. Sections applied by default are the clearest tell that a
--- description was shaped rather than written, and they make a small change look
--- like a large one.
---
--- LENGTH IS THE PROXY, not a judgement about content, because nothing here can
--- tell a section that earns its place from one that does not. A pull request
--- long enough to need signposting passes this without trying.
local SECTIONS_NEED = 1200

local headings = 0
for _ in string.gmatch(body, "##+ ") do
	headings += 1
end
if headings > 0 and #body < SECTIONS_NEED then
	table.insert(
		problems,
		`This body is {#body} characters and has {headings} heading(s). Under {SECTIONS_NEED} a reader gets through it in one pass, so say it plainly and drop the sections.`
	)
end

--- CONCATENATED, NOT JOINED. The scan only asks whether a byte is above
--- 127, so a separator between the two would add nothing to look at.
local at, character = style.nonAsciiAt(title .. body)
if at and character then
	table.insert(
		problems,
		`Remove the non-ASCII character '{character}'. Use a hyphen, a colon or a new sentence, and no emoji.`
	)
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

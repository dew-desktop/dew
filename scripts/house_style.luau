--!strict
--[[
	house_style -- the writing rules that are checkable, in one place.

	`docs/contributing/guides/writing.md` has said all of this in prose for as
	long as it has existed, and it was still violated across a whole pull request
	by a contributor who had read it. A rule nobody checks is a rule nobody
	follows, which is the same finding this repository has recorded about
	boundaries, dependencies and skipped tests.

	Used by `commitlint_check` for commit messages and `pr_check` for a pull
	request's title and body, so the two cannot drift.
]]

export type Finding = {
	rule: string,
	detail: string,
}

--- Tool attribution, which belongs in neither a commit nor a pull request.
---
--- `Co-Authored-By` MEANS "A PERSON TO CONTACT", which a tool is not, and a
--- trailer cannot be removed later without rewriting published history. A
--- generated-by footer does worse: it invites the reader to assume the prose
--- above it was not thought through, which is the opposite of what a commit
--- body is for.
local ATTRIBUTION = {
	{ pattern = "[Cc]o%-[Aa]uthored%-[Bb]y:.*[Cc]laude", detail = "a Co-Authored-By trailer naming a tool" },
	{ pattern = "[Cc]o%-[Aa]uthored%-[Bb]y:.*[Cc]opilot", detail = "a Co-Authored-By trailer naming a tool" },
	{ pattern = "[Cc]o%-[Aa]uthored%-[Bb]y:.*[Cc]ursor", detail = "a Co-Authored-By trailer naming a tool" },
	{ pattern = "[Cc]o%-[Aa]uthored%-[Bb]y:.*GPT", detail = "a Co-Authored-By trailer naming a tool" },
	{ pattern = "[Cc]o%-[Aa]uthored%-[Bb]y:.*noreply@anthropic", detail = "a Co-Authored-By trailer naming a tool" },
	{ pattern = "[Gg]enerated with", detail = "a generated-by footer" },
	{ pattern = "[Cc]laude [Cc]ode", detail = "a tool's name" },
	{ pattern = "claude%.com", detail = "a tool's link" },
	{ pattern = "[Aa][Ii]%-generated", detail = "a generated-by note" },
}

--- Vocabulary from the planning artifacts, which are gitignored.
---
--- `.artifacts/` IS NOT PUBLISHED, so "milestone 9 step B" names something the
--- reader cannot open. It reads as an instruction being restated rather than a
--- change being introduced, and it dates badly: the number means nothing once
--- the plan moves on, while what the work was FOR stays true.
---
--- Say the theme instead. Not "milestone 9 step B", but "an applet asks the host
--- for the surface it draws into".
local ORCHESTRATION = {
	{ pattern = "[Mm]ilestone", detail = "a milestone, which lives in gitignored planning notes" },
	{ pattern = "[Ss]print", detail = "a sprint, which lives in gitignored planning notes" },
	{ pattern = "[Aa][Dd][Rr]%-%d", detail = "an ADR number, which lives in gitignored planning notes" },
	{ pattern = "%.artifacts", detail = "a path under .artifacts, which is gitignored" },
	{ pattern = "[Ss]tep [A-E][%s%.,:;]", detail = "a plan step letter" },
	{ pattern = "[Ss]tep [A-E]$", detail = "a plan step letter" },
	{ pattern = "[Ss]tep %d", detail = "a plan step number" },
}

--- Subject openings that are not the imperative.
---
--- The rule is "fix the layer root", not "fixed", "fixes", or "a fix for". Past
--- tense describes what someone did; the imperative describes what the commit
--- does when applied, which is what a reader running `git log` is asking.
local NOT_IMPERATIVE = {
	"added", "adds", "adding",
	"fixed", "fixes", "fixing",
	"updated", "updates", "updating",
	"removed", "removes", "removing",
	"changed", "changes", "changing",
	"created", "creates", "creating",
	"implemented", "implements", "implementing",
	"refactored", "refactors", "refactoring",
	"moved", "moves", "moving",
	"renamed", "renames", "renaming",
	"made", "makes", "making",
	"improved", "improves", "improving",
	"introduced", "introduces", "introducing",
	"allowed", "allows", "allowing",
	"prevented", "prevents", "preventing",
	"ensured", "ensures", "ensuring",
	"bumped", "bumps", "bumping",
	"wrote", "writes", "writing",
	"landed", "lands", "landing",
	"switched", "switches", "switching",
	"teaches", "taught",
}

--- Openings that make the subject a statement rather than an instruction.
---
--- "a surface is a permission an applet asks for" is a true sentence and not a
--- commit subject. Leading with a determiner is the reliable tell.
local DECLARATIVE_OPENERS = {
	"a", "an", "the", "this", "that", "these", "those",
	"it", "its", "there", "we", "our", "you", "they", "their",
	--- Quantifiers open a noun phrase as reliably as an article does. "one pump
	--- for the thread" and "every applet declares its surface" are both true
	--- sentences and neither is an instruction.
	"one", "two", "three", "no", "none", "every", "each",
	"all", "both", "some", "any", "most", "many", "few",
	"nothing", "everything", "anything", "something",
}

local function has(list: { string }, word: string): boolean
	for _, entry in ipairs(list) do
		if entry == word then
			return true
		end
	end
	return false
end

local style = {}

--- Attribution and planning vocabulary, anywhere in a piece of text.
function style.findForbidden(text: string): { Finding }
	local found: { Finding } = {}
	for _, rule in ipairs(ATTRIBUTION) do
		if string.find(text, rule.pattern) then
			table.insert(found, { rule = "attribution", detail = rule.detail })
			break
		end
	end
	for _, rule in ipairs(ORCHESTRATION) do
		if string.find(text, rule.pattern) then
			table.insert(found, { rule = "orchestration", detail = rule.detail })
		end
	end
	return found
end

--- Is this subject written in the imperative?
---
--- Takes the text AFTER `type(scope): `, so the first word is the verb.
function style.subjectProblem(summary: string): string?
	local first = string.lower(string.match(summary, "^(%a[%w'-]*)") or "")
	if first == "" then
		return nil
	end
	if has(NOT_IMPERATIVE, first) then
		return `subject starts with '{first}'; use the imperative, as in "fix the layer root" rather than "fixes" or "fixed"`
	end
	if has(DECLARATIVE_OPENERS, first) then
		return `subject starts with '{first}', which makes it a statement rather than an instruction; say what applying the commit does`
	end
	return nil
end

style.NOT_IMPERATIVE = NOT_IMPERATIVE
style.DECLARATIVE_OPENERS = DECLARATIVE_OPENERS

return style

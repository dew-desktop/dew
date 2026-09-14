--!strict
--[[
	house_style -- the two writing rules this repository actually enforces.

	Authorship, because a trailer naming a tool claims a person to contact who
	does not exist, and it cannot be removed once published without rewriting
	history. And plain ASCII, which CONTRIBUTING has promised for commits, pull
	requests and documentation alike since before any of it was checked.

	EVERYTHING ABOUT VOICE IS DELIBERATELY ABSENT. Whether a subject reads as an
	instruction, how long a body runs, what shape a pull request takes: those are
	one maintainer's preferences, and a gate that fails a contributor's build over
	a preference is a house style with a lock on it.
	`docs/contributing/guides/writing.md` says what is preferred, and nothing here
	makes it a condition of landing work.
]]

export type Finding = {
	rule: string,
	detail: string,
}

local ATTRIBUTION = {
	{ pattern = "[Cc]o%-[Aa]uthored%-[Bb]y:.*[Cc]laude", detail = "a Co-Authored-By trailer naming a tool" },
	{ pattern = "[Cc]o%-[Aa]uthored%-[Bb]y:.*[Cc]opilot", detail = "a Co-Authored-By trailer naming a tool" },
	{ pattern = "[Cc]o%-[Aa]uthored%-[Bb]y:.*[Cc]ursor", detail = "a Co-Authored-By trailer naming a tool" },
	{ pattern = "[Cc]o%-[Aa]uthored%-[Bb]y:.*GPT", detail = "a Co-Authored-By trailer naming a tool" },
	{ pattern = "[Gg]enerated with", detail = "a generated-by footer" },
	{ pattern = "[Aa][Ii]%-generated", detail = "a generated-by note" },
}

local style = {}

--- Tool attribution, anywhere in a piece of text.
function style.findForbidden(text: string): { Finding }
	local found: { Finding } = {}
	for _, rule in ipairs(ATTRIBUTION) do
		if string.find(text, rule.pattern) then
			table.insert(found, { rule = "attribution", detail = rule.detail })
			break
		end
	end
	return found
end

--- Plain ASCII, which CONTRIBUTING promises for three places and had for one.
---
--- `verify_ascii` walks Markdown, so documentation was covered and commits were
--- not. A bare emoji in a subject passed every gate.
---
--- BYTE ABOVE 127, not a character allowlist. An em dash, a curly quote, an
--- arrow and an emoji are one rule, and naming them individually is how the next
--- one gets in.
function style.nonAsciiAt(text: string): (number?, string?)
	for index = 1, #text do
		local byte = string.byte(text, index)
		if byte and byte > 127 then
			--- Report the whole character, not its first byte, so the message
			--- shows what to delete rather than a number.
			local width = 1
			if byte >= 240 then
				width = 4
			elseif byte >= 224 then
				width = 3
			elseif byte >= 192 then
				width = 2
			end
			return index, string.sub(text, index, index + width - 1)
		end
	end
	return nil, nil
end

return style

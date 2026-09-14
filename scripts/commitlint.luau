--!strict
-- Dew Luau Commitlint Configuration & Validator (@dew/commitlint)
-- Strictly mapped to directories that physically exist in the repository

local style = require("./house_style")

export type CommitLintConfig = {
    types: {string},
    allowedScopes: {string},
    maxHeaderLength: number,
    enforceLowerCaseScope: boolean
}

export type ParsedCommit = {
    raw: string,
    type: string,
    scope: string?,
    summary: string
}

export type ValidationResult = {
    valid: boolean,
    errors: {string},
    parsed: ParsedCommit?
}

local config: CommitLintConfig = {
    --- Types that may omit a scope.
    ---
    --- A documentation or housekeeping change routinely spans the whole
    --- repository, and `docs(docs)` is not information. A `feat` or a `fix`
    --- always has somewhere it happened, so it always has to say where.
    scopeOptionalFor = {
        docs = true,
        chore = true,
        --- `ci` and `style` span the repository the same way. A workflow is not
        --- in a directory the scopes name, and `cargo fmt` touches whatever
        --- needed formatting; `ci(ci)` and `style(host)` are both worse than the
        --- bare type.
        ci = true,
        style = true,
    },

    types = {
        "feat",
        "fix",
        "perf",
        "refactor",
        "docs",
        "chore",
        "test",
        "ci"
    },
    allowedScopes = {
        "core",      -- host/src, the platform itself
        "host",      -- host/, build and wiring
        --- crates/ -- the rendering stack: raster, runtime, window. It arrived
        --- from Aether in milestone 2's first sprint (ADR-004) and is a
        --- top-level directory of this repository like any other, so it gets a
        --- scope like any other. `host` would have covered it and would have
        --- said the wrong thing: these are four packages now, not one with
        --- wiring.
        "crates",    -- crates/, the rendering stack
        "applets",      -- applets/
        "types",     -- types/
        "docs",      -- docs/, CONTRIBUTING, README
        "scripts",   -- scripts/
        "assets",    -- host/assets/
        "examples",  -- examples/, programs that are not mods
        --- demos/ -- applications that exercise a guest framework so
        --- `framework-coverage` can report how much of it is demonstrated rather
        --- than merely shipped. Separate from `examples` because an example is
        --- written to be read and a demo is written to be driven: it declares an
        --- interaction script and fails the build when the pixels stop moving.
        --- ADR-008 keeps them in this repository while the bootstrap lasts.
        "demos",     -- demos/, applications driven to prove a framework responds
        "deps"       -- VERSION, manifests, pins
    },
    maxHeaderLength = 100,
    enforceLowerCaseScope = true
}

local function parseCommit(message: string): (ParsedCommit?, string?)
    --- THE HEADER IS THE FIRST LINE, and it has to be cut out before matching.
    --- A Lua `.` matches a newline, so `(.+)$` against a whole message swallowed
    --- the body into the summary: the length rule then reported 756 for a
    --- 66-character subject, and the imperative rule read the wrong words.
    local header = string.match(message, "^%C+") or message

    local pattern = "^([%w_]+)%(([%w%-_/]+)%):%s+(.+)$"
    local ctype, scope, summary = string.match(header, pattern)

    if not ctype then
        local noScopePattern = "^([%w_]+):%s+(.+)$"
        ctype, summary = string.match(header, noScopePattern)
        if not ctype then
            return nil, "Message must follow format: <type>(<scope>): <summary>"
        end
        scope = nil
    end

    return {
        raw = header,
        type = ctype,
        scope = scope,
        summary = summary
    }, nil
end

local function validateCommit(message: string): ValidationResult
    local errors: {string} = {}
    local parsed, parseErr = parseCommit(message)

    if not parsed then
        table.insert(errors, parseErr or "Failed to parse commit header")
        return { valid = false, errors = errors, parsed = nil }
    end

    -- 1. Validate Type
    local isValidType = false
    for _, t in ipairs(config.types) do
        if parsed.type == t then
            isValidType = true
            break
        end
    end
    if not isValidType then
        table.insert(errors, `Invalid type: '{parsed.type}'. Expected one of: {table.concat(config.types, ", ")}`)
    end

    -- 2. Validate Scope (Must exist in allowedScopes)
    if parsed.scope then
        local isValidScope = false
        for _, s in ipairs(config.allowedScopes) do
            if parsed.scope == s or string.sub(parsed.scope, 1, #s + 1) == `{s}/` then
                isValidScope = true
                break
            end
        end
        if not isValidScope then
            table.insert(errors, `Invalid scope: '{parsed.scope}'. Scopes must strictly match existing repo directories: {table.concat(config.allowedScopes, ", ")}`)
        end

        if config.enforceLowerCaseScope and string.lower(parsed.scope) ~= parsed.scope then
            table.insert(errors, `Scope '{parsed.scope}' must be lower-case`)
        end
    elseif not config.scopeOptionalFor[parsed.type] then
        table.insert(errors, `Scope is required for '{parsed.type}' (e.g. feat(host): ...)`)
    end

    -- 3. Header Length
    if #parsed.raw > config.maxHeaderLength then
        table.insert(errors, `Commit header length ({#parsed.raw}) exceeds maximum ({config.maxHeaderLength})`)
    end

    -- 4. No attribution naming a tool.
    for _, finding in ipairs(style.findForbidden(message)) do
        table.insert(errors, `Remove {finding.detail}: a commit is authored by the person landing it`)
    end

    -- 5. Plain ASCII, which is promised for commits and was only checked in docs.
    local at, character = style.nonAsciiAt(message)
    if at and character then
        table.insert(
            errors,
            `Non-ASCII character '{character}' at byte {at}; use a hyphen, a colon or a new sentence, and no emoji`
        )
    end

    return {
        valid = #errors == 0,
        errors = errors,
        parsed = parsed
    }
end

local commitlint = {
    config = config,
    parse = parseCommit,
    validate = validateCommit
}

return commitlint

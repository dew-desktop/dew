--!strict
-- Dew Luau Commitlint Configuration & Validator (@dew/commitlint)
-- Strictly mapped to directories that physically exist in the repository

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
        "mods",      -- mods/
        "types",     -- types/
        "docs",      -- docs/, CONTRIBUTING, README
        "scripts",   -- scripts/
        "assets",    -- host/assets/
        "deps"       -- VERSION, manifests, pins
    },
    maxHeaderLength = 100,
    enforceLowerCaseScope = true
}

local function parseCommit(message: string): (ParsedCommit?, string?)
    local pattern = "^([%w_]+)%(([%w%-_/]+)%):%s+(.+)$"
    local ctype, scope, summary = string.match(message, pattern)

    if not ctype then
        local noScopePattern = "^([%w_]+):%s+(.+)$"
        ctype, summary = string.match(message, noScopePattern)
        if not ctype then
            return nil, "Message must follow format: <type>(<scope>): <summary>"
        end
        scope = nil
    end

    return {
        raw = message,
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

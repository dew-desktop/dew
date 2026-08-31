--!strict
-- Dew Version Bumper & Release Calculator (@dew/bump-version)
-- Calculates SemVer bumps, changelog entries, and release tags

export type BumpType = "prealpha" | "to-alpha" | "alpha-bump" | "patch" | "minor" | "major"

export type BumpPlan = {
    currentVersion: string,
    nextVersion: string,
    tag: string,
    changelogEntry: string
}

local function calculateBump(currentVersion: string, bumpType: BumpType, message: string?, dateStr: string?): BumpPlan
    local nextVersion = "0.0.1"
    local date = dateStr or "2026-08-26"
    local note = message or "Milestone release"

    if bumpType == "prealpha" then
        -- 0.0.X -> 0.0.(X+1)
        local p = string.match(currentVersion, "^0%.0%.(%d+)$")
        if p then
            nextVersion = `0.0.{tonumber(p) + 1}`
        else
            nextVersion = "0.0.1"
        end
    elseif bumpType == "to-alpha" then
        -- 0.0.X -> 0.1.0-alpha.1
        nextVersion = "0.1.0-alpha.1"
    elseif bumpType == "alpha-bump" then
        -- 0.1.0-alpha.X -> 0.1.0-alpha.(X+1)
        local base, num = string.match(currentVersion, "^(0%.%d+%.%d+%-alpha)%.(%d+)$")
        if base and num then
            nextVersion = `{base}.{tonumber(num) + 1}`
        else
            nextVersion = "0.1.0-alpha.1"
        end
    elseif bumpType == "patch" then
        local maj, min, pat = string.match(currentVersion, "^(%d+)%.(%d+)%.(%d+)")
        if maj and min and pat then
            nextVersion = `{maj}.{min}.{tonumber(pat) + 1}`
        else
            nextVersion = "0.1.1"
        end
    elseif bumpType == "minor" then
        local maj, min = string.match(currentVersion, "^(%d+)%.(%d+)%.(%d+)")
        if maj and min then
            nextVersion = `{maj}.{tonumber(min) + 1}.0`
        else
            nextVersion = "0.2.0"
        end
    elseif bumpType == "major" then
        local maj = string.match(currentVersion, "^(%d+)%.(%d+)%.(%d+)")
        if maj then
            nextVersion = `{tonumber(maj) + 1}.0.0`
        else
            nextVersion = "1.0.0"
        end
    end

    local tag = `v{nextVersion}`
    local changelogEntry = string.format("\n## [%s] - %s\n### Changes\n- %s\n", nextVersion, date, note)

    return {
        currentVersion = currentVersion,
        nextVersion = nextVersion,
        tag = tag,
        changelogEntry = changelogEntry
    }
end

local bumpVersion = {
    calculateBump = calculateBump
}

return bumpVersion

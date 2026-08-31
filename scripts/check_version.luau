--!strict
-- Dew Version Consistency Checker (@dew/check-version)
-- Validates SemVer 2.0.0 compliance and manifest synchronization

export type VersionCheckResult = {
    valid: boolean,
    canonicalVersion: string,
    errors: {string}
}

local function parseSemVer(version: string): boolean
    local pattern = "^(%d+)%.(%d+)%.(%d+)(-[%w%.%-_]+)?(%+[%w%.%-_]+)?$"
    local major, minor, patch = string.match(version, pattern)
    return major ~= nil and minor ~= nil and patch ~= nil
end

local function validateManifestVersion(canonicalVersion: string, manifestName: string, manifestContent: string): (boolean, string?)
    if not parseSemVer(canonicalVersion) then
        return false, `Canonical version '{canonicalVersion}' is not valid SemVer 2.0.0`
    end

    local matchPattern = 'version%s*=%s*"([^"]+)"'
    if string.find(manifestName, "%.json$") then
        matchPattern = '"version"%s*:%s*"([^"]+)"'
    end

    local foundVersion = string.match(manifestContent, matchPattern)
    if not foundVersion then
        return false, `Could not locate version key in {manifestName}`
    end

    if foundVersion ~= canonicalVersion then
        return false, `{manifestName} version ({foundVersion}) does not match canonical ({canonicalVersion})`
    end

    return true, nil
end

local checkVersion = {
    parseSemVer = parseSemVer,
    validateManifestVersion = validateManifestVersion
}

return checkVersion

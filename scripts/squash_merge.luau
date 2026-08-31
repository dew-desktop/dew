--!strict
-- Dew Squash-Merge Helper (@dew/squash-merge)
-- Generates clean git squash-merge execution plans with commit validation

local commitlint = require("./commitlint")

export type SquashMergePlan = {
    branch: string,
    commitMessage: string,
    gitCommands: {string},
    isValidMessage: boolean,
    validationErrors: {string}
}

local function planSquashMerge(branch: string, commitMessage: string): SquashMergePlan
    local valResult = commitlint.validate(commitMessage)

    local commands = {
        "git checkout main",
        `git merge --squash {branch}`,
        `git commit -m "{commitMessage}"`
    }

    return {
        branch = branch,
        commitMessage = commitMessage,
        gitCommands = commands,
        isValidMessage = valResult.valid,
        validationErrors = valResult.errors
    }
end

local squashMerge = {
    planSquashMerge = planSquashMerge
}

return squashMerge

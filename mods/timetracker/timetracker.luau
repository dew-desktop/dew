--!strict
-- Flagship Dew Mod: Time Tracker & Pomodoro HUD
local dew = require("@dew/core")

type TimeEntry = {
    id: string,
    task: string,
    project: string?,
    tag: string?,
    startTime: number,
    endTime: number?,
    duration: number
}

type TrackerState = {
    isRunning: boolean,
    isPomodoro: boolean,
    pomodoroPhase: "work" | "break",
    pomodoroRemaining: number,
    currentTask: string,
    currentProject: string,
    currentTag: string,
    elapsedSeconds: number,
    startTime: number,
    todayTotalSeconds: number,
    entries: {TimeEntry}
}

local state: TrackerState = {
    isRunning = false,
    isPomodoro = false,
    pomodoroPhase = "work",
    pomodoroRemaining = 25 * 60,
    currentTask = "Deep Focus",
    currentProject = "General",
    currentTag = "work",
    elapsedSeconds = 0,
    startTime = 0,
    todayTotalSeconds = 0,
    entries = {}
}

local function formatTimer(seconds: number): string
    local h = math.floor(seconds / 3600)
    local m = math.floor((seconds % 3600) / 60)
    local s = seconds % 60
    if h > 0 then
        return string.format("%02d:%02d:%02d", h, m, s)
    else
        return string.format("%02d:%02d", m, s)
    end
end

local function toggleTimer()
    state.isRunning = not state.isRunning
    if state.isRunning then
        state.startTime = dew.time.now()
        dew.audio.playChime("tick")
        dew.notifications.show(`⏱️ Started: {state.currentTask}`)
    else
        dew.audio.playChime("subtle")
        dew.notifications.show(`⏸️ Paused: {state.currentTask} ({formatTimer(state.elapsedSeconds)})`)
    end
    dew.hud.refresh()
end

-- Register Primary HUD Pill Widget
dew.hud.registerWidget({
    id = "timetracker-main",
    title = "Time Tracker",
    priority = 100,
    render = function()
        local displayText = ""
        local color = "#22c55e" -- green
        local progress = nil

        if state.isPomodoro then
            displayText = `{formatTimer(state.pomodoroRemaining)} ({state.pomodoroPhase})`
            color = if state.pomodoroPhase == "work" then "#ef4444" else "#3b82f6"
            progress = 1.0 - (state.pomodoroRemaining / (25 * 60))
        else
            displayText = formatTimer(state.elapsedSeconds)
            color = if state.isRunning then "#22c55e" else "#94a3b8"
        end

        return {
            text = displayText,
            subtext = state.currentTask,
            icon = if state.isRunning then "play" else "pause",
            color = color,
            glow = state.isRunning,
            badge = state.currentProject,
            progress = progress
        }
    end,
    onClick = function()
        toggleTimer()
    end,
    onSecondaryClick = function()
        dew.commandPalette.open({
            placeholder = "Switch task or log (e.g. 'Drafting docs #docs @client')...",
            onSubmit = function(query: string)
                if #query > 0 then
                    state.currentTask = query
                    state.elapsedSeconds = 0
                    state.isRunning = true
                    dew.hud.refresh()
                end
            end
        })
    end
})

-- Global Hotkeys
dew.hotkeys.register("Alt+Shift+T", toggleTimer)

dew.hotkeys.register("Alt+Shift+P", function()
    state.isPomodoro = not state.isPomodoro
    state.pomodoroPhase = "work"
    state.pomodoroRemaining = 25 * 60
    dew.notifications.show(if state.isPomodoro then "🍅 Pomodoro Mode ON" else "⏱️ Stopwatch Mode ON")
    dew.hud.refresh()
end)

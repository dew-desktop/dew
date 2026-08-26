--!strict
-- Flagship Dew Mod: Time Tracker & Pomodoro HUD
local dew = (_G.dew or require("@dew/core")) :: any

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
    lastTickTime: number,
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
    lastTickTime = 0,
    todayTotalSeconds = 0,
    entries = {}
}

local function formatTimer(seconds: number): string
    local total = math.max(0, math.floor(seconds))
    local h = math.floor(total / 3600)
    local m = math.floor((total % 3600) / 60)
    local s = total % 60
    if h > 0 then
        return string.format("%02d:%02d:%02d", h, m, s)
    else
        return string.format("%02d:%02d", m, s)
    end
end

local function toggleTimer()
    state.isRunning = not state.isRunning
    local now = dew.time.now()
    if state.isRunning then
        state.startTime = now
        state.lastTickTime = now
        dew.audio.playChime("tick")
        dew.notifications.show(`⏱️ Started: {state.currentTask}`)
    else
        dew.audio.playChime("subtle")
        dew.notifications.show(`⏸️ Paused: {state.currentTask} ({formatTimer(state.elapsedSeconds)})`)
    end
    dew.hud.refresh()
end

-- Primary HUD Pill Widget
dew.hud.registerWidget({
    id = "timetracker-main",
    title = "Time Tracker",
    priority = 100,
    render = function()
        local now = dew.time.now()
        if state.isRunning and state.lastTickTime > 0 then
            local dt = now - state.lastTickTime
            if dt > 0 then
                state.elapsedSeconds += dt
                state.lastTickTime = now
                if state.isPomodoro then
                    state.pomodoroRemaining -= dt
                    if state.pomodoroRemaining <= 0 then
                        state.pomodoroPhase = if state.pomodoroPhase == "work" then "break" else "work"
                        state.pomodoroRemaining = if state.pomodoroPhase == "work" then 25 * 60 else 5 * 60
                        dew.audio.playChime("complete")
                        dew.notifications.show(`🍅 Pomodoro: {string.upper(state.pomodoroPhase)} interval started!`)
                    end
                end
            end
        end

        local displayText = ""
        local color = "#22c55e" -- vibrant emerald green
        local progress = nil

        if state.isPomodoro then
            displayText = `{formatTimer(state.pomodoroRemaining)}`
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
        state.isPomodoro = not state.isPomodoro
        state.pomodoroPhase = "work"
        state.pomodoroRemaining = 25 * 60
        dew.notifications.show(if state.isPomodoro then "🍅 Pomodoro Mode" else "⏱️ Stopwatch Mode")
        dew.hud.refresh()
    end
})

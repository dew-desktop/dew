--!strict
-- Flagship Dew Native Mod: Time Tracker HUD (Aether Vello GPU Presentation)
local dew = (_G.dew or require("@dew/core")) :: any

type TrackerState = {
    isRunning: boolean,
    isPomodoro: boolean,
    pomodoroPhase: "work" | "break",
    pomodoroRemaining: number,
    currentTask: string,
    currentProject: string,
    elapsedSeconds: number,
    lastTickTime: number
}

local state: TrackerState = {
    isRunning = false,
    isPomodoro = false,
    pomodoroPhase = "work",
    pomodoroRemaining = 25 * 60,
    currentTask = "Deep Focus",
    currentProject = "DEV",
    elapsedSeconds = 0,
    lastTickTime = 0
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
        state.lastTickTime = now
        dew.audio.playChime("tick")
        dew.notifications.show(`⏱️ Started: {state.currentTask}`)
    else
        dew.audio.playChime("subtle")
        dew.notifications.show(`⏸️ Paused: {state.currentTask} ({formatTimer(state.elapsedSeconds)})`)
    end
    dew.hud.refresh()
end

dew.hud.registerComponent({
    id = "timetracker",
    render = function()
        local now = dew.time.now()
        if state.isRunning and state.lastTickTime > 0 then
            local dt = now - state.lastTickTime
            if dt > 0 then
                state.elapsedSeconds += dt
                state.lastTickTime = now
            end
        end

        local displayTime = formatTimer(state.elapsedSeconds)
        local dotColor = if state.isRunning then { r = 16, g = 185, b = 129, a = 255 } else { r = 100, g = 116, b = 139, a = 255 }

        return {
            commands = {
                -- 1. Glowing Indicator Dot
                {
                    kind = "rounded_rect",
                    x = 24.0,
                    y = 28.0,
                    w = 8.0,
                    h = 8.0,
                    radius = 4.0,
                    r = dotColor.r,
                    g = dotColor.g,
                    b = dotColor.b,
                    a = dotColor.a
                },
                -- 2. Project Badge Pill
                {
                    kind = "rounded_rect",
                    x = 42.0,
                    y = 23.0,
                    w = 48.0,
                    h = 18.0,
                    radius = 9.0,
                    r = 255,
                    g = 255,
                    b = 255,
                    a = 20
                },
                -- 3. Card Separator
                {
                    kind = "rounded_rect",
                    x = 260.0,
                    y = 18.0,
                    w = 1.0,
                    h = 28.0,
                    radius = 0.0,
                    r = 255,
                    g = 255,
                    b = 255,
                    a = 25
                },
                -- 4. Quick Action Button Pill
                {
                    kind = "rounded_rect",
                    x = 276.0,
                    y = 18.0,
                    w = 78.0,
                    h = 28.0,
                    radius = 14.0,
                    r = if state.isRunning then 16 else 255,
                    g = if state.isRunning then 185 else 255,
                    b = if state.isRunning then 129 else 255,
                    a = if state.isRunning then 40 else 20
                }
            }
        }
    end,
    onClick = function()
        toggleTimer()
    end
})

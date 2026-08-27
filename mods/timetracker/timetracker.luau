--!strict
-- Flagship Dew Native Mod: Time Tracker HUD (Aether Roblox UI Parity DSL)
local dew = (_G.dew or require("@dew/core")) :: any
local Aether = (_G.Aether or require("@dew/aether")) :: any

local New = Aether.New
local UDim2 = Aether.UDim2
local UDim = Aether.UDim
local Color3 = Aether.Color3
local Enum = Aether.Enum

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
        local statusColor = if state.isRunning then Color3.fromHex("#10b981") else Color3.fromHex("#64748b")
        local textColor = if state.isRunning then Color3.fromHex("#10b981") else Color3.fromHex("#f8fafc")

        -- Pure Roblox UI Tree Construction
        return New "Frame" {
            Size = UDim2.new(1, 0, 1, 0),
            BackgroundTransparency = 1,

            -- 1. Glowing Indicator Dot
            New "Frame" {
                Position = UDim2.new(0, 20, 0, 24),
                Size = UDim2.new(0, 8, 0, 8),
                BackgroundColor3 = statusColor,
                BackgroundTransparency = 0,

                New "UICorner" {
                    CornerRadius = UDim.new(0, 4)
                }
            },

            -- 2. Project Badge Pill
            New "Frame" {
                Position = UDim2.new(0, 36, 0, 18),
                Size = UDim2.new(0, 46, 0, 20),
                BackgroundColor3 = Color3.fromRGB(255, 255, 255),
                BackgroundTransparency = 0.9,

                New "UICorner" {
                    CornerRadius = UDim.new(0, 10)
                },

                New "TextLabel" {
                    Position = UDim2.new(0, 9, 0, 2),
                    Size = UDim2.new(1, 0, 1, 0),
                    Text = state.currentProject,
                    TextSize = 11,
                    TextColor3 = Color3.fromHex("#cbd5e1"),
                    BackgroundTransparency = 1
                }
            },

            -- 3. Center Info: Monospace Timer & Subtitle Task
            New "TextLabel" {
                Position = UDim2.new(0, 98, 0, 12),
                Size = UDim2.new(0, 120, 0, 22),
                Text = displayTime,
                TextSize = 18,
                Font = Enum.Font.Code,
                TextColor3 = textColor,
                BackgroundTransparency = 1
            },
            New "TextLabel" {
                Position = UDim2.new(0, 99, 0, 33),
                Size = UDim2.new(0, 120, 0, 16),
                Text = state.currentTask,
                TextSize = 10.5,
                TextColor3 = Color3.fromHex("#94a3b8"),
                BackgroundTransparency = 1
            },

            -- 4. Card Separator
            New "Frame" {
                Position = UDim2.new(0, 268, 0, 16),
                Size = UDim2.new(0, 1, 0, 24),
                BackgroundColor3 = Color3.fromRGB(255, 255, 255),
                BackgroundTransparency = 0.88
            },

            -- 5. Quick Action Button Pill
            New "TextButton" {
                Position = UDim2.new(0, 282, 0, 14),
                Size = UDim2.new(0, 76, 0, 28),
                BackgroundColor3 = if state.isRunning then Color3.fromHex("#10b981") else Color3.fromRGB(255, 255, 255),
                BackgroundTransparency = if state.isRunning then 0.8 else 0.9,

                New "UICorner" {
                    CornerRadius = UDim.new(0, 14)
                },

                New "TextLabel" {
                    Position = UDim2.new(0, if state.isRunning then 20 else 22, 0, 6),
                    Size = UDim2.new(1, 0, 1, 0),
                    Text = if state.isRunning then "PAUSE" else "START",
                    TextSize = 11,
                    TextColor3 = textColor,
                    BackgroundTransparency = 1
                }
            }
        }
    end,
    onClick = function()
        toggleTimer()
    end
})

--!strict
-- Flagship Dew Native Mod: Time Tracker HUD (Vide Reactivity + Aether Roblox UI Parity)
local dew = (_G.dew or require("@dew/core")) :: any
local Aether = (_G.Aether or require("@dew/aether")) :: any

local New = Aether.New
local UDim2 = Aether.UDim2
local UDim = Aether.UDim
local Color3 = Aether.Color3
local Enum = Aether.Enum
local source = Aether.source
local derive = Aether.derive

-- 1. Pure Vide Reactive State (Zero Framework Leakage)
local isRunning = source(false)
local elapsedSeconds = source(0)
local currentTask = source("Deep Focus")
local currentProject = source("DEV")
local lastTickTime = 0

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

-- 2. Derived Reactive Computations
local displayTime = derive(function()
    return formatTimer(elapsedSeconds())
end)

local statusColor = derive(function()
    return if isRunning() then Color3.fromHex("#10b981") else Color3.fromHex("#64748b")
end)

local textColor = derive(function()
    return if isRunning() then Color3.fromHex("#10b981") else Color3.fromHex("#f8fafc")
end)

local buttonLabel = derive(function()
    return if isRunning() then "PAUSE" else "START"
end)

local buttonBg = derive(function()
    return if isRunning() then Color3.fromHex("#10b981") else Color3.fromRGB(255, 255, 255)
end)

local buttonTrans = derive(function()
    return if isRunning() then 0.8 else 0.9
end)

local function toggleTimer()
    local running = not isRunning()
    isRunning(running)

    local now = dew.time.now()
    if running then
        lastTickTime = now
        dew.audio.playChime("tick")
        dew.notifications.show(`⏱️ Started: {currentTask()}`)
    else
        dew.audio.playChime("subtle")
        dew.notifications.show(`⏸️ Paused: {currentTask()} ({displayTime()})`)
    end
    dew.hud.refresh()
end

-- 3. Component Declaration using pure Roblox UI Tree + Vide bindings
dew.hud.registerComponent({
    id = "timetracker",
    render = function()
        local now = dew.time.now()
        if isRunning() and lastTickTime > 0 then
            local dt = now - lastTickTime
            if dt > 0 then
                elapsedSeconds(elapsedSeconds() + dt)
                lastTickTime = now
            end
        end

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
                    Text = currentProject,
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
                Text = currentTask,
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
                BackgroundColor3 = buttonBg,
                BackgroundTransparency = buttonTrans,

                New "UICorner" {
                    CornerRadius = UDim.new(0, 14)
                },

                New "TextLabel" {
                    Position = function()
                        return UDim2.new(0, if isRunning() then 20 else 22, 0, 6)
                    end,
                    Size = UDim2.new(1, 0, 1, 0),
                    Text = buttonLabel,
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

--!strict
--[[
	Desktop Clock: a minimalist desktop clock applet for Dew.

	Features:
	- Sleek vertical gradient matching Dew's water-droplet branding
	- Automatically detects system timezone via `desktop.Time`
	- Click anywhere to toggle between 12-hour and 24-hour format
	- Formats hours without leading zeros (e.g. 9:39:00 instead of 09:39:00)
	- Draggable positioning (`draggable = true`) and edge snapping (`snapToEdges = true`)
	- Saves window position and format preference across launches (`savePosition = true`, `desktop.Storage`)
]]

local widget = assert(desktop.Widget, "dew.toml must grant `widget`")

local root = widget({
	width = 200,
	height = 44,
	anchor = "top-right",
	offset = { x = 24, y = 24 },
	draggable = true,
	savePosition = true,
	snapToEdges = true,
	keepOnScreen = true,
})

-- Load format preference from persistent storage (default: 12-hour)
local is24Hour = false
if desktop.Storage and desktop.Storage.get("clock.format") == "24h" then
	is24Hour = true
end

-- Full-surface click target with Dew-branded vertical gradient background
local button = Instance.new("TextButton")
button.Name = "ClockSurface"
button.Size = UDim2.fromScale(1, 1)
button.BackgroundColor3 = Color3.fromRGB(255, 255, 255)
button.BackgroundTransparency = 0
button.Text = ""
button.AutoButtonColor = false
button.Parent = root

-- Rounded corners
local corner = Instance.new("UICorner")
corner.CornerRadius = UDim.new(0, 10)
corner.Parent = button

-- Vertical gradient matching Dew's water-droplet branding (sky blue to periwinkle violet)
local gradient = Instance.new("UIGradient")
gradient.Rotation = 90
gradient.Color = ColorSequence.new(
	Color3.fromRGB(72, 148, 226), -- Dew sky/ocean blue
	Color3.fromRGB(132, 120, 218) -- Dew lilac/periwinkle violet
)
gradient.Parent = button

-- Subtle highlight border
local borderStroke = Instance.new("UIStroke")
borderStroke.Thickness = 1.5
borderStroke.Color = Color3.fromRGB(210, 232, 255)
borderStroke.Transparency = 0.3
borderStroke.Parent = button

-- Digital time readout
local timeLabel = Instance.new("TextLabel")
timeLabel.Name = "TimeDisplay"
timeLabel.Size = UDim2.fromScale(1, 1)
timeLabel.BackgroundTransparency = 1
timeLabel.TextColor3 = Color3.fromRGB(255, 255, 255)
timeLabel.TextSize = 26
timeLabel.TextXAlignment = Enum.TextXAlignment.Center
timeLabel.TextYAlignment = Enum.TextYAlignment.Center
timeLabel.Parent = button

local textStroke = Instance.new("UIStroke")
textStroke.Thickness = 1
textStroke.Color = Color3.fromRGB(30, 48, 80)
textStroke.Transparency = 0.5
textStroke.Parent = timeLabel

-- Time formatting helper
local function updateClock()
	local hours: number
	local minutes: number
	local seconds: number

	if (desktop.Time :: any).local_time then
		local t = (desktop.Time :: any).local_time()
		hours = t.hour
		minutes = t.min
		seconds = t.sec
	elseif (desktop.Time :: any).timezone_offset then
		local secOfDay = math.floor(desktop.Time.now() + (desktop.Time :: any).timezone_offset()) % 86400
		if secOfDay < 0 then
			secOfDay = secOfDay + 86400
		end
		hours = math.floor(secOfDay / 3600)
		minutes = math.floor((secOfDay % 3600) / 60)
		seconds = secOfDay % 60
	else
		-- Fallback timezone offset (-7 for Pacific / local time)
		local tzOffset = -7
		if desktop.Storage and desktop.Storage.get("clock.tz") then
			tzOffset = tonumber(desktop.Storage.get("clock.tz")) or -7
		end
		local secOfDay = math.floor(desktop.Time.now() + (tzOffset * 3600)) % 86400
		if secOfDay < 0 then
			secOfDay = secOfDay + 86400
		end
		hours = math.floor(secOfDay / 3600)
		minutes = math.floor((secOfDay % 3600) / 60)
		seconds = secOfDay % 60
	end

	if is24Hour then
		-- No leading zero on hours
		timeLabel.Text = string.format("%d:%02d:%02d", hours, minutes, seconds)
	else
		local ampm = if hours >= 12 then "PM" else "AM"
		local h12 = hours % 12
		if h12 == 0 then
			h12 = 12
		end
		-- No leading zero on hours
		timeLabel.Text = string.format("%d:%02d:%02d %s", h12, minutes, seconds, ampm)
	end
end

-- Toggle format on click and persist preference
button.MouseButton1Click:Connect(function()
	is24Hour = not is24Hour
	if desktop.Storage then
		desktop.Storage.set("clock.format", if is24Hour then "24h" else "12h")
	end
	updateClock()
end)

-- Initial paint and per-frame update subscription
updateClock()
desktop.Clock.OnFrame(updateClock)

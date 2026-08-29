--!strict
--[[
	Time Tracker — a Dew widget.

	A DECLARATION. Requiring this file defines a table and does nothing else; the
	host reads `id` and `size`, checks `mod.json`, builds the capability table,
	and only then calls `mount`. See docs/mod_contract.md.

	Everything below `mount` is ORDINARY AETHER — `create`, `source`, `derive`,
	and Roblox's property names. There is no Dew dialect, which is what keeps the
	tree liftable: it can be mounted in a Roblox place or rendered by
	`aether snapshot` without a line changing.
]]

local Aether = require("@aether/api")

local create = Aether.create
local source = Aether.source
local derive = Aether.derive

local function formatTimer(seconds: number): string
	local total = math.max(0, math.floor(seconds))
	local h = math.floor(total / 3600)
	local m = math.floor((total % 3600) / 60)
	local s = total % 60
	if h > 0 then
		return string.format("%02d:%02d:%02d", h, m, s)
	end
	return string.format("%02d:%02d", m, s)
end

return {
	id = "timetracker",
	size = { width = 380, height = 56 },

	--- `dew` carries exactly what mod.json declared: storage, audio,
	--- notifications. Not a global, and not the whole host surface.
	mount = function(dew: any)
		local isRunning = source(false)
		local elapsed = source(0)
		local task = source("Deep Focus")
		local project = source("DEV")

		--- DERIVED, not recomputed by the host each frame. The old shape rebuilt
		--- the whole tree from a `render` callback every frame, which made these
		--- decorative — the rebuild did the work. Now a write to `elapsed` is
		--- what reaches the screen, the same way it would in an engine.
		local display = derive(function()
			return formatTimer(elapsed())
		end)

		local accent = derive(function()
			return if isRunning() then Color3.fromHex("#10b981") else Color3.fromHex("#64748b")
		end)

		local buttonLabel = derive(function()
			return if isRunning() then "PAUSE" else "START"
		end)

		local function toggle()
			local running = not isRunning()
			isRunning(running)
			if dew.audio then
				dew.audio.chime(if running then "tick" else "subtle")
			end
			if dew.notifications then
				dew.notifications.show(
					if running then `Started: {task()}` else `Paused: {task()} ({display()})`
				)
			end
		end

		--- Advanced by the host's clock rather than read from a wall clock inside
		--- a render pass. `dew.time` is ungated; what is gated is everything that
		--- CHANGES something.
		local lastTick = dew.time.now()
		local function tick()
			local now = dew.time.now()
			if isRunning() then
				elapsed(elapsed() + (now - lastTick))
			end
			lastTick = now
		end

		local tree = create "Frame" {
			Name = "TimeTracker",
			Size = UDim2.new(1, 0, 1, 0),
			BackgroundColor3 = Color3.fromHex("#121722"),

			create "Frame" {
				Name = "Status",
				Position = UDim2.new(0, 20, 0, 24),
				Size = UDim2.new(0, 8, 0, 8),
				BackgroundColor3 = accent,

				create "UICorner" { CornerRadius = UDim.new(0, 4) },
			},

			create "TextLabel" {
				Name = "Project",
				Position = UDim2.new(0, 36, 0, 18),
				Size = UDim2.new(0, 46, 0, 20),
				BackgroundColor3 = Color3.fromHex("#1e2635"),
				Text = project,
				TextSize = 11,
				TextColor3 = Color3.fromHex("#cbd5e1"),

				create "UICorner" { CornerRadius = UDim.new(0, 10) },
			},

			create "TextLabel" {
				Name = "Timer",
				Position = UDim2.new(0, 98, 0, 12),
				Size = UDim2.new(0, 120, 0, 22),
				BackgroundTransparency = 1,
				Text = display,
				TextSize = 18,
				TextColor3 = accent,
			},

			create "TextLabel" {
				Name = "Task",
				Position = UDim2.new(0, 98, 0, 32),
				Size = UDim2.new(0, 140, 0, 14),
				BackgroundTransparency = 1,
				Text = task,
				TextSize = 11,
				TextColor3 = Color3.fromHex("#94a3b8"),
			},

			create "TextLabel" {
				Name = "Toggle",
				Position = UDim2.new(0, 286, 0, 16),
				Size = UDim2.new(0, 74, 0, 24),
				BackgroundColor3 = accent,
				Text = buttonLabel,
				TextSize = 12,
				TextColor3 = Color3.fromHex("#0b0f16"),

				create "UICorner" { CornerRadius = UDim.new(0, 12) },
			},
		}

		--- Handed back beside the tree, the way the counter example does it: the
		--- widget owns its state and exposes the transitions, and the host owns
		--- what CAUSES them — a hotkey, a click, a frame.
		return tree, { toggle = toggle, tick = tick }
	end,
}

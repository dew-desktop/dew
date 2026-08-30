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

	--- A FLOATING WIDGET. No chrome, always on top, out of the taskbar, and
	--- shaped by its own alpha — so the rounded card below is the window's real
	--- silhouette rather than a shape drawn on a dark rectangle.
	---
	--- Omitting `surface` entirely gets this too; it is spelled out here because
	--- this is the reference mod and the fields should be visible somewhere.
	surface = {
		kind = "widget",
		anchor = "top-right",
		offset = { x = 24, y = 24 },
		--- False, because the START button is a control. A monitor that only
		--- displays would set this true and let clicks reach the desktop.
		clickThrough = false,
	},

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

		--- SUBSCRIBED TO FRAMES, rather than handed back for the host to call.
		---
		--- `tick` used to be returned beside the tree with `toggle`, on the theory
		--- that the widget owns state and the host owns what causes a transition.
		--- That is right for a HOTKEY and wrong for a clock: nothing in Dew was
		--- calling it, so the timer sat at 00:00 forever.
		---
		--- `Clock.OnFrame` is the same seam Roblox's Heartbeat fills, and
		--- `Live.Session` already steps it once per frame — so this is the engine's
		--- own frame signal on both hosts rather than a Dew convention.
		local clock = Aether.Host.detect().Clock
		local lastTick = dew.time.now()
		clock.OnFrame(function()
			local now = dew.time.now()
			if isRunning() then
				elapsed(elapsed() + (now - lastTick))
			end
			lastTick = now
		end)

		local tree = create "Frame" {
			Name = "TimeTracker",
			Size = UDim2.new(1, 0, 1, 0),
			BackgroundColor3 = Color3.fromHex("#121722"),

			--- THE WINDOW'S SHAPE, not a decoration on it. On a transparent
			--- surface the corners outside this radius are pixels the window does
			--- not occupy: the desktop shows through them and receives the click.
			create "UICorner" { CornerRadius = UDim.new(0, 14) },

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

			--- A PRESSABLE, not a TextLabel that looks like one.
			---
			--- This was a plain label for its whole life: it rendered as a button,
			--- `toggle` was defined and returned beside the tree, and nothing ever
			--- called it. Which is a perfectly convincing button until you click
			--- it, and looks exactly like an input bug in the host.
			Aether.Pressable({
				Name = "Toggle",
				Position = UDim2.new(0, 286, 0, 16),
				Size = UDim2.new(0, 74, 0, 24),
				BackgroundColor3 = accent,
				--- A Pressable is a hit area and defaults to invisible; anything
				--- that should be seen opts in.
				BackgroundTransparency = 0,

				--- ACTIVATED, not Pressed: a press that leaves the button before
				--- releasing should not count, which is what every other button on
				--- the desktop does.
				OnActivated = toggle,

				create "UICorner" { CornerRadius = UDim.new(0, 12) },

				create "TextLabel" {
					Name = "Label",
					Size = UDim2.new(1, 0, 1, 0),
					BackgroundTransparency = 1,
					Text = buttonLabel,
					TextSize = 12,
					TextColor3 = Color3.fromHex("#0b0f16"),
				},
			}),
		}

		--- Handed back beside the tree, the way the counter example does it: the
		--- widget owns its state and exposes the transitions, and the host owns
		--- what CAUSES them — a hotkey, a click, a frame.
		--- `toggle` is still exposed: a hotkey should be able to start the
		--- timer without the pointer, and that genuinely is the host's to
		--- cause. `tick` is gone — it drives itself.
		return tree, { toggle = toggle }
	end,
}

--!strict
--[[
	Workspace — a window manager inside a window.

	THE POINT OF THIS MOD IS THAT DEW DID NOT CHANGE FOR IT. Dew hands out one
	OS window; everything below is ordinary Aether, so the same tree runs in a
	Roblox place with no host at all. Two window managers at different scales, the
	way a terminal multiplexer runs inside a real one.

	It is a scale model of `spektr/window_manager` rather than a port of it. That
	package is 1046 lines with zero dependencies — genuinely portable — but it is
	still in a monorepo no consumer can pin, and porting it would have proved that
	the monorepo is reachable rather than that the SHAPE works off-engine. This
	proves the shape: virtual windows, dragged, stacked, and hit-tested by
	Aether's own router, with no engine underneath.

	WHAT IT DELIBERATELY LACKS: docking, split panes, resize, minimise. Those are
	the other 900 lines and none of them would test anything this does not.
]]

local Aether = require("@aether/api")
local PointerRouter = require("@aether/PointerRouter")
--- The pointer's current position. `PointerRouter` routes presses; `InputScope`
--- is where "where is the pointer right now" actually lives.
local InputScope = require("@aether/InputScope")

local create = Aether.create
local source = Aether.source
local derive = Aether.derive

--- Z-ORDER FROM AETHER, not from this mod.
---
--- `LayerManager.CreateStack` is the same allocator the Studio window manager
--- uses, and it lives in the framework precisely so a stacking rule is not
--- reinvented per application. Depth is ALLOCATED rather than written: a window
--- asks to be in front and is told what number that is.
local stack = Aether.LayerManager.CreateStack(100, 10)

local TITLEBAR_HEIGHT = 26

type WindowProps = {
	title: string,
	x: number,
	y: number,
	width: number,
	height: number,
	accent: any,
	body: string,
}

local function VirtualWindow(props: WindowProps)
	local x = source(props.x)
	local y = source(props.y)
	local z = source(100)

	--- The stack writes back through this, so bringing one window forward
	--- renumbers the others without any of them knowing about each other.
	local handle = {}
	stack.Register(handle, function(depth: number)
		z(depth)
	end)

	--- DRAG STATE, held outside the reactive graph on purpose. These are read and
	--- written within a single pointer event and nothing renders from them, so
	--- making them sources would subscribe every frame to values that are not
	--- part of the picture.
	local dragging = false
	local grabX, grabY = 0, 0

	--- GLOBAL POINTER, not a titlebar event. A drag continues while the pointer
	--- is anywhere — including outside the window it started in, which is most of
	--- the interesting part of a drag. `Pressable` reports the press that STARTS
	--- it; the router reports the movement that continues it.
	PointerRouter.OnPointerMove(function(px: number, py: number)
		if dragging then
			x(px - grabX)
			y(py - grabY)
		end
	end)

	PointerRouter.OnPointerUp(function()
		dragging = false
	end)

	local titlebar = Aether.Pressable({
		Name = "Titlebar",
		Size = UDim2.new(1, 0, 0, TITLEBAR_HEIGHT),
		Position = UDim2.new(0, 0, 0, 0),
		BackgroundColor3 = props.accent,
		--- OPT IN TO BEING VISIBLE. A `Pressable` defaults to
		--- `BackgroundTransparency = 1` because the primitive is a hit area, not a
		--- surface — it has no opinion about how it looks. A titlebar does.
		BackgroundTransparency = 0,

		OnPressed = function()
			stack.BringToFront(handle)
			dragging = true
			--- The grab offset keeps the pointer on the same spot of the titlebar
			--- for the whole drag. Without it the window jumps so its corner meets
			--- the cursor the moment it is picked up.
			local px, py = InputScope.GetPointerPosition()
			grabX = px - x()
			grabY = py - y()
		end,

		OnReleased = function()
			dragging = false
		end,

		create "TextLabel" {
			Name = "Title",
			Size = UDim2.new(1, -16, 1, 0),
			Position = UDim2.new(0, 10, 0, 0),
			BackgroundTransparency = 1,
			Text = props.title,
			TextSize = 12,
			TextColor3 = Color3.fromHex("#0b0f16"),
			TextXAlignment = Enum.TextXAlignment.Left,
		},
	})

	return create "Frame" {
		Name = props.title,
		Position = derive(function()
			return UDim2.fromOffset(x(), y())
		end),
		Size = UDim2.fromOffset(props.width, props.height),
		BackgroundColor3 = Color3.fromHex("#1a2130"),
		ZIndex = z,
		--- CORRECT AUTHORING, AND CURRENTLY A NO-OP HERE. On Roblox this plus the
		--- `UICorner` below rounds the titlebar's top corners with the frame's.
		--- Off-engine it does not: `Layout` honours `ClipsDescendants` and
		--- intersects the rects, but `Live.Frame` carries a clip as a bare
		--- rectangle with no radius, so the painter has nothing to round to.
		---
		--- Left in because it is what the tree MEANS, and because this is a
		--- divergence between the two hosts rather than a quirk of this mod —
		--- which is exactly the kind of thing the proof was for.
		ClipsDescendants = true,

		create "UICorner" { CornerRadius = UDim.new(0, 8) },

		titlebar,

		create "TextLabel" {
			Name = "Body",
			Position = UDim2.new(0, 12, 0, TITLEBAR_HEIGHT + 10),
			Size = UDim2.new(1, -24, 0, 18),
			BackgroundTransparency = 1,
			Text = props.body,
			TextSize = 12,
			TextColor3 = Color3.fromHex("#94a3b8"),
			TextXAlignment = Enum.TextXAlignment.Left,
		},
	}
end

return {
	id = "workspace",
	size = { width = 720, height = 440 },

	--- AN ORDINARY WINDOW, not a floating widget, and this is the one mod for
	--- which that is right: it is a desktop, so it wants to be a thing on the
	--- desktop rather than a thing floating over it. It is also the first
	--- exercise of the window surface from Dew's side.
	surface = { kind = "window", title = "Dew Workspace" },

	mount = function(_dew: any)
		--- `ScreenSpace` in the Studio package's sense: the full-bleed container
		--- every virtual window is positioned inside. Its own coordinates are the
		--- windows' coordinate system.
		return create "Frame" {
			Name = "ScreenSpace",
			Size = UDim2.new(1, 0, 1, 0),
			BackgroundColor3 = Color3.fromHex("#0d1117"),

			VirtualWindow({
				title = "Editor",
				x = 40,
				y = 40,
				width = 300,
				height = 150,
				accent = Color3.fromHex("#38bdf8"),
				body = "drag my titlebar",
			}),

			VirtualWindow({
				title = "Console",
				x = 170,
				y = 150,
				width = 300,
				height = 150,
				accent = Color3.fromHex("#f59e0b"),
				body = "press to raise",
			}),

			VirtualWindow({
				title = "Inspector",
				x = 320,
				y = 240,
				width = 300,
				height = 150,
				accent = Color3.fromHex("#a78bfa"),
				body = "stacked by LayerManager",
			}),
		}
	end,
}

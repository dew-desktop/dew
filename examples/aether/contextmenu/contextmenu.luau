--!strict
--[[
	A context menu opened by a right-click and closed by choosing a row.

	WHAT THIS DEMONSTRATES THAT A PRESS DOES NOT. `pressable` proves one pointer
	down reaches one rectangle. A menu is the next question up: a gesture opens a
	surface that was not there, the pointer travels across rows that light up as
	it passes, and the row under it when the button goes down is the one that
	runs. Every part of that is arbitration, and none of it can be read off a
	pixel count -- an open menu and a menu open on the wrong row paint almost the
	same image.

	So this exports `Opened` and `Chosen`, and the tool prints both. "The menu
	opened" and "the menu landed on Copy" are separate claims and the second is
	the one worth making.

	THE RIGHT BUTTON COMES FROM THE HOST, NOT FROM THE FRAMEWORK. Aether's
	`PointerRouter` arbitrates the primary button and has no opinion about the
	secondary one, which is correct -- what a right-click MEANS is an
	application's decision, and this application's decision is "summon the menu
	here". Dew reports the button through `DewHost.Input`, the same service it
	reports the primary one through, so the gesture below is a real one the host
	delivered rather than a call this file made to itself.

	THE FRAMEWORK COMES FROM THIS DIRECTORY'S OWN INSTALL: `pesde install` here
	puts Aether and vide in `roblox_packages/`, and the require below goes
	through the redirect pesde writes. Dew's host injects nothing, and this
	applet asks the host for the surface it draws into.
]]

local Aether = require("./roblox_packages/aether")

local WIDTH, HEIGHT = 240, 180

--- Where the right-click lands, and therefore where the menu's top-left corner
--- goes. Both the tree and the `Script` read these, so the press cannot drift
--- from the thing it is aimed at.
local CLICK = { x = 40, y = 30 }
local MENU = { width = 132, row = 26, padding = 6, spacing = 3 }

local ROWS = {
	{ Id = "cut", Label = "Cut" },
	{ Id = "copy", Label = "Copy" },
	{ Id = "paste", Label = "Paste" },
}

--- The centre of row `index`, in surface coordinates: the padding above the
--- first row, then a row and a gap for each one before it.

local opens = 0
local chosen: string? = nil

--- What the right-click writes to. Set by `build`, because the sources belong to
--- the tree being built and the host's input service is installed once.
local summon: ((x: number, y: number) -> ())? = nil

--- Built with whatever Aether it is handed, so the coverage proxy can stand in
--- for the real table. Every symbol this reads is a symbol this demo shows.
local function build(A: any): any
	local open = A.source(false)
	local anchor = A.source(Vector2.new(CLICK.x, CLICK.y))
	local picked = A.source(nil :: string?)

	summon = function(x: number, y: number)
		opens += 1
		anchor(Vector2.new(x, y))
		open(true)
	end

	local items = {}
	for _, row in ipairs(ROWS) do
		table.insert(items, {
			Id = row.Id,
			Label = row.Label,
			OnActivated = function()
				chosen = row.Id
				picked(row.Id)
			end,
		})
	end

	return A.create "Frame" {
		Name = "Root",
		Size = UDim2.fromOffset(WIDTH, HEIGHT),

		--- THE CHOICE IS ON THE BACKGROUND, so the last paint differs from the
		--- first after the menu has closed again. A menu that opens and shuts
		--- leaves the surface exactly as it found it, and a differential over
		--- that reads zero for an interaction that worked perfectly.
		BackgroundColor3 = function()
			return if picked() ~= nil
				then Color3.new(0.16, 0.28, 0.2)
				else Color3.new(0.07, 0.08, 0.11)
		end,

		--- WHAT WAS CHOSEN, ON SCREEN. A test beside this file reads the tree the
		--- host holds, the way a person reads the window, so an outcome that is
		--- not drawn is an outcome nothing can check.
		A.create "TextLabel" {
			Name = "Chosen",
			Position = UDim2.fromOffset(12, HEIGHT - 28),
			Size = UDim2.fromOffset(WIDTH - 24, 20),
			BackgroundTransparency = 1,
			TextColor3 = Color3.new(0.85, 0.89, 0.95),
			TextSize = 13,
			TextXAlignment = Enum.TextXAlignment.Left,
			Text = function()
				return `chosen: {picked() or "nothing"}`
			end,
		},

		A.ContextMenu({
			IsOpen = open,
			OnClose = function()
				open(false)
			end,
			AnchorPosition = anchor,
			Items = function()
				return items
			end,
			Position = UDim2.fromOffset(CLICK.x, CLICK.y),
			Size = UDim2.fromOffset(MENU.width, 0),
		}),
	}
end

--- THE APPLET ASKS FOR ITS SURFACE and parents its own tree into it. Nothing
--- here is handed back for a host to mount on this applet's behalf.
local root = (dew :: any).widget({ width = WIDTH, height = HEIGHT })

local mounted = Aether.Desktop.Mount(function()
	return build(Aether)
end, WIDTH, HEIGHT)

;(mounted.Tree :: any).Parent = root

--- THE HOST'S OWN INPUT SERVICE, connected once per process the way a framework
--- connects to it. `MouseButton2` is the only thing this listens for; the
--- primary button belongs to the router and arrives through the session.
local host: any = (DewHost :: any)
if host ~= nil and host.Input ~= nil then
	host.Input.InputBegan:Connect(function(input: any)
		if tostring(input.UserInputType) ~= "Enum.UserInputType.MouseButton2" then
			return
		end
		if summon then
			summon(input.Position.X, input.Position.Y)
		end
	end)
end


--!strict
--[[
	A pressable that changes colour while pressed.

	THE SMALLEST HONEST DIFFERENTIAL. A press is one pointer down inside a
	rectangle, and the colour it drives is a property the rasteriser reads -- so
	"the feature responded" and "the pixels moved" are the same sentence here.
	Nothing about this needs a delay, a scheduler or a layer surface, which is
	what makes it the right first demo rather than the most interesting one.

	THE FRAMEWORK COMES FROM THIS DIRECTORY'S OWN INSTALL (ADR-008): `pesde
	install` here puts Aether and vide in `roblox_packages/`, and the require
	below goes through the redirect pesde writes. Dew's host injects nothing.
]]

local Aether = require("./roblox_packages/aether")

local WIDTH, HEIGHT = 200, 100
local BUTTON = { x = 20, y = 20, width = 120, height = 44 }

local presses = 0

--- Built with whatever Aether it is handed, so the coverage proxy can stand in
--- for the real table. Every symbol this reads is a symbol this demo shows.
local function build(A: any): any
	local pressed = A.source(false)

	return A.create "Frame" {
		Name = "Root",
		Size = UDim2.fromOffset(WIDTH, HEIGHT),
		BackgroundColor3 = Color3.new(0.07, 0.08, 0.11),

		A.Pressable({
			Name = "Button",
			Position = UDim2.fromOffset(BUTTON.x, BUTTON.y),
			Size = UDim2.fromOffset(BUTTON.width, BUTTON.height),

			--- OPAQUE, EXPLICITLY. `Pressable` defaults
			--- `BackgroundTransparency` to 1, so a button given only a colour
			--- paints nothing -- and a differential over it reads zero for a
			--- feature that is working. `host/tests/fixtures/pressable.luau` sets
			--- this for the same reason.
			BackgroundTransparency = 0,

			--- A DERIVED COLOUR, so the differential turns on the press rather
			--- than on anything the host happened to repaint.
			BackgroundColor3 = function()
				return if pressed()
					then Color3.new(0.95, 0.75, 0.2)
					else Color3.new(0.22, 0.55, 0.95)
			end,

			--- `OnPressed` AND `OnReleased` ARE THE CONTRACT. There is no
			--- `OnPressedChanged`; the first version of this demo invented one and
			--- the press changed nothing, which looked exactly like a broken
			--- Pressable.
			OnPressed = function()
				presses += 1
				pressed(true)
			end,

			OnReleased = function()
				pressed(false)
			end,
		}),
	}
end

local mounted = Aether.Desktop.Mount(function()
	return build(Aether)
end, WIDTH, HEIGHT)

--- PARENTED HERE ONLY IF A ROOT IS ALREADY IN SCOPE.
---
--- `framework-coverage` installs a `DewRoot` global and never calls `mount`, so
--- the tree has to reach the surface at load for it. `dew` calls `mount` and
--- hands the root in, which is the branch below.
if (DewRoot :: any) ~= nil then
	(mounted.Tree :: any).Parent = (DewRoot :: any)
end

return {
	id = "pressable",

	--- `size` FOR THE HOST, `Width`/`Height` FOR THE TOOL. Two consumers reading
	--- the same two numbers under different names; without `size` the host
	--- rendered this at a default and the image was the wrong shape.
	size = { width = WIDTH, height = HEIGHT },

	Session = mounted.Session,
	Width = WIDTH,
	Height = HEIGHT,

	--- WHAT `dew` CALLS. The tree is already built, so this only parents it into
	--- the root the host made. An applet that mounts its own framework needs
	--- nothing else from the host.
	mount = function(_dew: any, root: any)
		(mounted.Tree :: any).Parent = root
		return mounted.Tree
	end,

	--- DOWN AND HELD. The press is left down so the last paint differs from the
	--- first: releasing would restore the colour and the differential would read
	--- zero for a feature that worked perfectly.
	Script = {
		{ step = 1 / 60 },
		{ pointer = "down", x = BUTTON.x + BUTTON.width / 2, y = BUTTON.y + BUTTON.height / 2 },
		{ step = 1 / 60 },
	},

	Presses = function(): number
		return presses
	end,

	Aether = Aether,
	Measure = function(A: any)
		return Aether.Desktop.Mount(function()
			return build(A)
		end, WIDTH, HEIGHT)
	end,
}

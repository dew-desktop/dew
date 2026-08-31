--!strict
--[[
	Dew's mount prelude.

	The three lines a desktop host must run that Roblox does not, kept in ONE
	place rather than repeated per mod. A mod declares `mount`; this is what turns
	the tree that returns into something the frame loop can drive.

	Every mod's VM loads this before its own module, so a mod file contains no
	host ceremony at all — which is what keeps the tree it builds liftable into a
	Roblox place unchanged.
]]

local Aether = require("@aether/api")
local Live = require("@aether/host/Live")
local PointerRouter = require("@aether/PointerRouter")

local host = Aether.Host.detect()

--- `UDim2` and `Color3` as globals, because there is no engine to define them
--- and a mod names them exactly as a Roblox component does.
host.InstallVocabulary()

return {
	Host = host,

	--- Build a mod's tree INSIDE a reactive scope, then open a session over it.
	---
	--- THE SCOPE IS WHY THIS TAKES A FUNCTION rather than a finished tree. vide
	--- refuses `derive` and `effect` outside a stable scope — "cannot use derive()
	--- outside a stable or reactive scope" — and a mod's `mount` is full of both.
	--- Calling it from the host and handing the result over means it ran outside
	--- one; calling it in here means every mod gets the same scope, on the same
	--- terms, without a line of ceremony in the mod itself.
	---
	--- `host.root`, NOT `host.mount`. The headless host's `mount` simply calls the
	--- component and parents the result — it opens no scope, because in Roblox
	--- vide's own `mount` is what does that. `root` is the reactive core's, is
	--- present on both hosts, and is the thing that actually establishes a scope.
	Mount = function(build: (any) -> (any, any), dew: any, width: number, height: number)
		local tree: any
		local actions: any

		host.root(function()
			tree, actions = build(dew)
			return tree
		end)

		return {
			Session = Live.Session(host, tree, PointerRouter, width, height),
			--- Whatever the mod handed back beside its tree: the transitions the
			--- host may drive from a hotkey, a click or a frame.
			Actions = actions or {},
		}
	end,
}

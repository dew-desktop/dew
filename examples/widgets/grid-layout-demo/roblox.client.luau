--!strict
--[[
	The Roblox entry point for grid-layout-demo, for `rojo serve` or `rojo build`
	run on this folder.

	default.project.json puts grid-layout-demo.luau in ReplicatedStorage as a
	ModuleScript named GridLayoutDemo. Requiring it once builds the demo, and the demo
	parents itself into this player's PlayerGui.
]]

require(game:GetService("ReplicatedStorage"):WaitForChild("GridLayoutDemo"))

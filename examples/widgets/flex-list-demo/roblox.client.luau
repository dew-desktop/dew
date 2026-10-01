--!strict
--[[
	The Roblox entry point for flex-list-demo, for `rojo serve` or `rojo build`
	run on this folder.

	default.project.json puts flex-list-demo.luau in ReplicatedStorage as a
	ModuleScript named FlexListDemo. Requiring it once builds the demo, and the demo
	parents itself into this player's PlayerGui.
]]

require(game:GetService("ReplicatedStorage"):WaitForChild("FlexListDemo"))

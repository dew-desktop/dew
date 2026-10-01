--!strict
--[[
	The Roblox entry point for flex-item-demo, for `rojo serve` or `rojo build`
	run on this folder.

	default.project.json puts flex-item-demo.luau in ReplicatedStorage as a
	ModuleScript named FlexItemDemo. Requiring it once builds the demo, and the demo
	parents itself into this player's PlayerGui.
]]

require(game:GetService("ReplicatedStorage"):WaitForChild("FlexItemDemo"))

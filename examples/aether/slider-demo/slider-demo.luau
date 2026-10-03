--!strict
--[[
	Slider demo -- milestone 60 sprint B4's own live consumer.

	A real Slider primitive (src/primitives/Slider.luau): createSlider's own
	clamp/step pipeline, a real track measured off the real instance, a real
	thumb bound through createDraggable's own bind shape. Dragged with a real
	mouse, this proves the thumb moves proportionally and the released
	position holds -- the exact bar milestone 58 already set for
	Scrollbar's own thumb.

	WHAT TO LOOK FOR, screenshotted:
	  1. rest     -- the thumb sits at its InitialValue position, the value
	                readout reads that value.
	  2. dragged  -- mid-drag, the thumb has moved proportionally toward
	                wherever the pointer is along the track.
	  3. released -- the thumb HOLDS the dragged-to position; it does not
	                snap back to where it started.
]]

local Aether = require("./roblox_packages/aether")
local vide = require("./roblox_packages/vide")

local WIDTH, HEIGHT = 320, 140

local root = desktop.Widget({ width = WIDTH, height = HEIGHT, anchor = "center" })

vide.root(function()
	local label = Instance.new("TextLabel")
	label.Name = "ValueLabel"
	label.Size = UDim2.new(1, -32, 0, 24)
	label.Position = UDim2.new(0, 16, 0, 16)
	label.BackgroundTransparency = 1
	label.TextColor3 = Color3.fromRGB(220, 220, 220)
	label.TextXAlignment = Enum.TextXAlignment.Left
	label.TextSize = 14
	label.Text = "value: 20"
	label.Parent = root

	Aether.Slider({
		Parent = root,
		Name = "DemoSlider",
		Position = UDim2.new(0, 16, 0, 64),
		Size = UDim2.new(1, -32, 0, 20),
		Min = 0,
		Max = 100,
		Step = 5,
		InitialValue = 20,
		TrackColor = Color3.fromRGB(40, 44, 58),
		ThumbColor = Color3.fromRGB(120, 160, 230),
		OnChange = function(v: number)
			label.Text = `value: {v}`
		end,
	})
end)

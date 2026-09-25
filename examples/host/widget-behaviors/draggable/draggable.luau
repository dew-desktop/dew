--!strict
--[[
	Draggable -- the base behavior every other example in this folder builds
	on: click and hold the widget's body to move it.

	    cargo run -- examples/host/widget-behaviors/draggable

	No title bar, the way a Rainmeter skin works -- the whole card is the drag
	handle. Nothing clamps it to the screen and nothing remembers where you
	left it; `keep-on-screen` and `save-position`, right beside this one, are
	those.
]]

local vide = require("./roblox_packages/vide")

local widget = assert(desktop.Widget, "dew.toml must grant `widget`")

local root = widget({
	width = 260,
	height = 120,
	anchor = "topRight",
	offset = { x = 24, y = 24 },
	zOrder = "topmost",
	draggable = true,
})

local card = Instance.new("Frame")
card.Name = "Card"
card.Size = UDim2.new(1, 0, 1, 0)
card.BackgroundColor3 = Color3.fromRGB(28, 22, 38)
card.Parent = root

local corner = Instance.new("UICorner")
corner.CornerRadius = UDim.new(0, 12)
corner.Parent = card

local title = Instance.new("TextLabel")
title.Name = "Title"
title.Size = UDim2.new(1, -32, 0, 30)
title.Position = UDim2.new(0, 16, 0, 16)
title.BackgroundTransparency = 1
title.Text = "Click and drag me"
title.TextColor3 = Color3.fromRGB(240, 233, 249)
title.TextSize = 20
title.TextXAlignment = Enum.TextXAlignment.Left
title.Parent = card

local note = Instance.new("TextLabel")
note.Name = "Note"
note.Size = UDim2.new(1, -32, 0, 60)
note.Position = UDim2.new(0, 16, 0, 52)
note.BackgroundTransparency = 1
note.Text = "This card can go anywhere, including off screen -- see keep-on-screen for the version that will not let you."
note.TextColor3 = Color3.fromRGB(180, 162, 200)
note.TextSize = 12
note.TextXAlignment = Enum.TextXAlignment.Left
note.TextWrapped = true
note.Parent = card

-- THE SOURCE IS THE ONLY STATE; THE EFFECT IS THE ONLY PLACE THAT REACTS.
-- No `wasDragging` to compare by hand -- `source` already ignores a write
-- that repeats its current value, so the effect below re-runs exactly when
-- dragging actually starts or stops, not once a frame.
local dragging = vide.source(false)

vide.root(function()
	vide.effect(function()
		title.Text = if dragging() then "Weee!" else "Click and drag me"
	end)
end)

-- STILL A POLL, AND STILL HAS TO BE: `Dragging` is an attribute (see
-- surface.rs), and an attribute has no changed signal to push through here
-- yet -- only `desktop.Clock.OnFrame` learns about it each frame. What moved to
-- vide is everything AFTER that: deciding whether it changed, and what to do
-- if it did.
desktop.Clock.OnFrame(function()
	dragging(root:GetAttribute("Dragging") == true)
end)

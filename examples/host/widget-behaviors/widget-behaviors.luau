--!strict
--[[
	Widget Behaviors -- the desktop-native behaviors a Rainmeter-shaped widget
	gets on top of `dew.Widget`'s original width/height/anchor/offset/clickThrough.

	    cargo run -- examples/host/widget-behaviors

	Still no framework here, matching `examples/host/basic-widget`'s
	convention for the TREE: `Instance.new`, property assignment and `Parent`
	are the whole vocabulary for building it. `vide`'s `source`/`effect` are
	the one exception, for reacting to the host's own `Dragging` attribute --
	a plain reactive primitive, not Aether, and not a reason this stops being
	a framework-free applet.

	Drag this card by its body -- there is no title bar, the way a Rainmeter
	skin works. Drag it near a screen edge and it snaps; try to drag it fully
	off screen and it will not go. Close it and run this again: it reopens
	where you left it rather than back at its declared anchor.
]]

local vide = require("./roblox_packages/vide")

local widget = assert(dew.Widget, "dew.toml must grant `widget`")

local root = widget({
	width = 260,
	height = 140,
	anchor = "topRight",
	offset = { x = 24, y = 24 },
	zOrder = "topmost",
	clickThrough = false,
	draggable = true,
	keepOnScreen = true,
	snapToEdges = true,
	savePosition = true,
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
note.Size = UDim2.new(1, -32, 0, 80)
note.Position = UDim2.new(0, 16, 0, 52)
note.BackgroundTransparency = 1
note.Text = "Drag this card by its body (there is no title bar)."
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
-- yet -- only `dew.Clock.OnFrame` learns about it each frame. What moved to
-- vide is everything AFTER that: deciding whether it changed, and what to do
-- if it did.
dew.Clock.OnFrame(function()
	dragging(root:GetAttribute("Dragging") == true)
end)

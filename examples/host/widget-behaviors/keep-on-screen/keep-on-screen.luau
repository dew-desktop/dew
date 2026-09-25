--!strict
--[[
	Keep On Screen -- `draggable`'s clamp: the widget can be pushed right up
	to the edge of the display, never fully off it.

	    cargo run -- examples/host/widget-behaviors/keep-on-screen

	Framework-free, like `examples/host/basic-widget` -- `Instance.new`,
	property assignment and `Parent` are the whole vocabulary.
]]

local widget = assert(desktop.Widget, "dew.toml must grant `widget`")

local root = widget({
	width = 260,
	height = 120,
	anchor = "topRight",
	offset = { x = 24, y = 24 },
	zOrder = "topmost",
	draggable = true,
	keepOnScreen = true,
})

local card = Instance.new("Frame")
card.Name = "Card"
card.Size = UDim2.new(1, 0, 1, 0)
card.BackgroundColor3 = Color3.fromRGB(22, 34, 30)
card.Parent = root

local corner = Instance.new("UICorner")
corner.CornerRadius = UDim.new(0, 12)
corner.Parent = card

local title = Instance.new("TextLabel")
title.Name = "Title"
title.Size = UDim2.new(1, -32, 0, 30)
title.Position = UDim2.new(0, 16, 0, 16)
title.BackgroundTransparency = 1
title.Text = "Keep On Screen"
title.TextColor3 = Color3.fromRGB(233, 249, 240)
title.TextSize = 20
title.TextXAlignment = Enum.TextXAlignment.Left
title.Parent = card

local note = Instance.new("TextLabel")
note.Name = "Note"
note.Size = UDim2.new(1, -32, 0, 60)
note.Position = UDim2.new(0, 16, 0, 52)
note.BackgroundTransparency = 1
note.Text = "Drag this card toward any edge of your screen. It stops before going fully off it."
note.TextColor3 = Color3.fromRGB(162, 200, 180)
note.TextSize = 12
note.TextXAlignment = Enum.TextXAlignment.Left
note.TextWrapped = true
note.Parent = card

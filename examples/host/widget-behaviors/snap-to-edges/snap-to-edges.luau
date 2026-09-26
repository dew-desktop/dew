--!strict
--[[
	Snap To Edges -- drag this card near a screen edge and release; it snaps
	to sit flush against it rather than leaving a gap.

	    cargo run -- examples/host/widget-behaviors/snap-to-edges

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
	snapToEdges = true,
})

local card = Instance.new("Frame")
card.Name = "Card"
card.Size = UDim2.new(1, 0, 1, 0)
card.BackgroundColor3 = Color3.fromRGB(22, 28, 38)
card.Parent = root

local corner = Instance.new("UICorner")
corner.CornerRadius = UDim.new(0, 12)
corner.Parent = card

local title = Instance.new("TextLabel")
title.Name = "Title"
title.Size = UDim2.new(1, -32, 0, 30)
title.Position = UDim2.new(0, 16, 0, 16)
title.BackgroundTransparency = 1
title.Text = "Snap To Edges"
title.TextColor3 = Color3.fromRGB(233, 239, 249)
title.TextSize = 20
title.TextXAlignment = Enum.TextXAlignment.Left
title.Parent = card

local note = Instance.new("TextLabel")
note.Name = "Note"
note.Size = UDim2.new(1, -32, 0, 60)
note.Position = UDim2.new(0, 16, 0, 52)
note.BackgroundTransparency = 1
note.Text = "Drag this card near any edge of your screen and let go. It snaps flush against it."
note.TextColor3 = Color3.fromRGB(146, 162, 188)
note.TextSize = 12
note.TextXAlignment = Enum.TextXAlignment.Left
note.TextWrapped = true
note.Parent = card

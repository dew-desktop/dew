--!strict
--[[
	Click Through -- `clickThrough`, the property every other example in this
	folder leaves off. This card is fully visible, but the window itself never
	receives a click: try clicking on it, and whatever is underneath it (your
	desktop, another window) gets the click instead.

	    cargo run -- examples/host/widget-behaviors/click-through

	It is not draggable, on purpose: dragging needs a press ON the widget to
	arm, and a click-through widget never sees one.

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
	clickThrough = true,
})

local card = Instance.new("Frame")
card.Name = "Card"
card.Size = UDim2.new(1, 0, 1, 0)
card.BackgroundColor3 = Color3.fromRGB(30, 22, 38)
card.Parent = root

local corner = Instance.new("UICorner")
corner.CornerRadius = UDim.new(0, 12)
corner.Parent = card

local title = Instance.new("TextLabel")
title.Name = "Title"
title.Size = UDim2.new(1, -32, 0, 30)
title.Position = UDim2.new(0, 16, 0, 16)
title.BackgroundTransparency = 1
title.Text = "Click Through"
title.TextColor3 = Color3.fromRGB(242, 233, 249)
title.TextSize = 20
title.TextXAlignment = Enum.TextXAlignment.Left
title.Parent = card

local note = Instance.new("TextLabel")
note.Name = "Note"
note.Size = UDim2.new(1, -32, 0, 60)
note.Position = UDim2.new(0, 16, 0, 52)
note.BackgroundTransparency = 1
note.Text = "Try clicking or dragging this card. Nothing happens here -- the click reaches whatever is behind it instead."
note.TextColor3 = Color3.fromRGB(180, 162, 200)
note.TextSize = 12
note.TextXAlignment = Enum.TextXAlignment.Left
note.TextWrapped = true
note.Parent = card

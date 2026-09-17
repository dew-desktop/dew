--!strict
--[[
	Widget Behaviors -- the desktop-native behaviors a Rainmeter-shaped widget
	gets on top of `dew.Widget`'s original width/height/anchor/offset/clickThrough.

	    cargo run -- examples/host/widget-behaviors

	There is no framework here, matching `examples/host/basic-widget`'s
	convention: `Instance.new`, property assignment and `Parent` are the whole
	vocabulary.

	Drag this card by its body -- there is no title bar, the way a Rainmeter
	skin works. Drag it near a screen edge and it snaps; try to drag it fully
	off screen and it will not go. Close it and run this again: it reopens
	where you left it rather than back at its declared anchor.
]]

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
title.Text = "Drag me"
title.TextColor3 = Color3.fromRGB(240, 233, 249)
title.TextSize = 20
title.TextXAlignment = Enum.TextXAlignment.Left
title.Parent = card

local note = Instance.new("TextLabel")
note.Name = "Note"
note.Size = UDim2.new(1, -32, 0, 80)
note.Position = UDim2.new(0, 16, 0, 52)
note.BackgroundTransparency = 1
note.Text = "Grab the body to move it. It snaps near a screen edge, cannot be "
	.. "dragged off screen, and remembers where you left it the next time "
	.. "this runs."
note.TextColor3 = Color3.fromRGB(180, 162, 200)
note.TextSize = 12
note.TextXAlignment = Enum.TextXAlignment.Left
note.TextWrapped = true
note.Parent = card

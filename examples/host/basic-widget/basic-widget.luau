--!strict
--[[
	Basic Widget -- the smallest applet that draws something.

	    cargo run -- examples/host/basic-widget

	A widget is a borderless panel that floats above your other windows, which is
	the shape most Dew applets take. This one opens one, fills it with a card, and
	puts two lines of text on it.

	There is no framework here. `Instance.new`, property assignment and `Parent`
	are the whole vocabulary, and every line would build the same tree on any host
	implementing the DataModel.
]]

--- `desktop` is a global, and it carries exactly the capabilities `dew.toml`
--- granted. `desktop.Widget` is nil unless the manifest asked for `widget`, so this
--- line turns a missing permission into a sentence naming the word to add.
local widget = assert(desktop.Widget, "dew.toml must grant `widget`")

--- Opening the surface hands back the instance to build under.
local root = widget({
	width = 260,
	height = 120,
	--- Corner of the screen to sit in, and how far in from it.
	anchor = "topRight",
	offset = { x = 24, y = 24 },
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
title.Position = UDim2.new(0, 16, 0, 18)
title.BackgroundTransparency = 1
title.Text = "Basic Widget"
title.TextColor3 = Color3.fromRGB(233, 239, 249)
title.TextSize = 20
title.TextXAlignment = Enum.TextXAlignment.Left
title.Parent = card

local note = Instance.new("TextLabel")
note.Name = "Note"
note.Size = UDim2.new(1, -32, 0, 40)
note.Position = UDim2.new(0, 16, 0, 54)
note.BackgroundTransparency = 1
note.Text = "260 by 120, top right corner"
note.TextColor3 = Color3.fromRGB(146, 162, 188)
note.TextSize = 12
note.TextXAlignment = Enum.TextXAlignment.Left
note.TextWrapped = true
note.Parent = card

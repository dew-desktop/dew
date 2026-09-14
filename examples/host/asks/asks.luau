--!strict
--[[
	Asks -- an applet that asks the host for a surface and returns nothing.

	    cargo run -- examples/host/asks

	The older shape returns a table the host reads: a `size`, a `surface` block
	and a `mount` for the host to call back into. That made the applet describe
	itself to something that then acted on its behalf, and the description had to
	carry every question the host might ask.

	Here the applet asks and is answered. `dew.widget` exists because `dew.toml`
	grants `widget`, and it hands back the root to parent into, so there is
	nothing left to return.
]]

--- `dew` IS A GLOBAL, the way `game` is one in the engine Dew is shaped after,
--- and it carries exactly the capabilities `dew.toml` granted.
---
--- `assert` RATHER THAN A BARE CALL, because `dew.widget` is nil when the
--- manifest did not ask for a widget, and that is the capability model rather
--- than an oversight. Naming the missing word here turns a nil call into a
--- sentence that says what to add.
local widget = assert(dew.widget, "dew.toml must grant `widget`")

local root = widget({
	width = 260,
	height = 120,
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
title.Text = "I'm a widget!"
title.TextColor3 = Color3.fromRGB(233, 239, 249)
title.TextSize = 20
title.TextXAlignment = Enum.TextXAlignment.Left
title.Parent = card

local note = Instance.new("TextLabel")
note.Name = "Note"
note.Size = UDim2.new(1, -32, 0, 40)
note.Position = UDim2.new(0, 16, 0, 54)
note.BackgroundTransparency = 1
note.Text = "no size, no surface block, no mount"
note.TextColor3 = Color3.fromRGB(146, 162, 188)
note.TextSize = 12
note.TextXAlignment = Enum.TextXAlignment.Left
note.TextWrapped = true
note.Parent = card

--!strict
--[[
	Flex item demo: a chat pane whose rows are UIListLayouts, where a
	UIFlexItem or a UISizeConstraint on one child decides who takes the spare
	width, who gives it back, and where each one stops.

	    cargo run -- examples/widgets/flex-item-demo

	Plain DataModel: `Instance.new`, properties and `Activated`, nothing imported,
	so the same file builds the same tree in a Roblox place. No child is given a
	computed width. Each one states a basis, and the flex items and constraints
	on it do the rest whenever the pane changes size.

	The pieces, and the one property each depends on:

	  sidebar      40% of the pane, capped by a UISizeConstraint MaxSize of 140
	  messages     UIFlexItem Grow: whatever width the sidebar leaves
	  bubbles      full width of the messages, capped at 240 by a MaxSize
	  field        UIFlexItem Grow: whatever width the composer row leaves
	  channel chip UIFlexItem Shrink: gives width back when the row is too
	               narrow, and a MinSize of 64 stops it before the name is lost
	  Send         UIFlexItem Custom, GrowRatio 0 until "Send ratio" raises it

	WHAT TO LOOK FOR, one click at a time from the start:

	  1. at rest       the pane is 440 wide. The sidebar has stopped at 140 and
	                   the messages take the other 294. The field stretches
	                   from the + button to the chip, and Send keeps its 60.
	  2. Field mode    the field switches to Custom with GrowRatio 2, and
	     then Send     Send ratio gives Send a GrowRatio of 1: the 160 the row
	     ratio         has spare is split two to one, so the field ends at
	                   about 187 and Send widens to about 113.
	  3. Narrower      each click takes 80 off the pane. At 280 the composer is
	                   exactly its children's width and nothing flexes. At 200
	                   the sidebar is back to 40% (80) and the messages keep
	                   their 100 plus the 14 left over; in the composer the chip
	                   shrinks until its MinSize holds it at 64, and the row,
	                   still 54 too wide, runs under the pane's right edge.
	  4. Wider         back out again: the chip returns to 90 once there is
	                   room, and the sidebar stops growing at 140 for good.

	The line under the pane always says what the controls are set to.
]]

local widget = assert(desktop.Widget, "dew.toml must grant `widget`")

local WIDTH, HEIGHT = 480, 320

--- The widths the pane steps through, narrowest first.
local PANE_MIN, PANE_MAX, PANE_STEP = 200, 440, 80

local root = widget({ width = WIDTH, height = HEIGHT })

local TEXT = Color3.fromRGB(226, 232, 244)
local MUTED = Color3.fromRGB(140, 152, 176)
local ACCENT = Color3.fromRGB(70, 120, 210)

local MESSAGES = {
	{ from = "ada", text = "the sidebar stops at 140 now" },
	{ from = "lin", text = "and the field takes the rest" },
	{ from = "ada", text = "try making the pane narrower" },
	{ from = "lin", text = "the chip holds at 64" },
}

local function rounded(parent: Instance, radius: number)
	local corner = Instance.new("UICorner")
	corner.CornerRadius = UDim.new(0, radius)
	corner.Parent = parent
end

local function inset(parent: Instance, x: number, y: number)
	local padding = Instance.new("UIPadding")
	padding.PaddingLeft = UDim.new(0, x)
	padding.PaddingRight = UDim.new(0, x)
	padding.PaddingTop = UDim.new(0, y)
	padding.PaddingBottom = UDim.new(0, y)
	padding.Parent = parent
end

--- A list in `parent`, in LayoutOrder.
---
--- SORTORDER DEFAULTS TO NAME, not LayoutOrder, on every layout class. A list
--- that leaves it unset is ordered alphabetically by child name.
local function list(parent: Instance, direction: Enum.FillDirection, gap: number): UIListLayout
	local layout = Instance.new("UIListLayout")
	layout.FillDirection = direction
	layout.SortOrder = Enum.SortOrder.LayoutOrder
	layout.Padding = UDim.new(0, gap)
	layout.Parent = parent
	return layout
end

--- How `child` shares its list's spare room or its overflow.
local function flex(child: Instance, mode: Enum.UIFlexMode): UIFlexItem
	local item = Instance.new("UIFlexItem")
	item.FlexMode = mode
	item.Parent = child
	return item
end

--- Bounds on `child`'s size, whatever a layout or a flex item would give it.
--- Pass `math.huge` for "no bound on this axis".
local function bounds(child: Instance, min: Vector2, max: Vector2)
	local constraint = Instance.new("UISizeConstraint")
	constraint.MinSize = min
	constraint.MaxSize = max
	constraint.Parent = child
end

local function label(parent: Instance, name: string, order: number, size: UDim2, text: string): TextLabel
	local l = Instance.new("TextLabel")
	l.Name = name
	l.LayoutOrder = order
	l.Size = size
	l.BackgroundTransparency = 1
	l.TextColor3 = TEXT
	l.TextSize = 13
	l.TextXAlignment = Enum.TextXAlignment.Left
	l.Text = text
	l.Parent = parent
	return l
end

local card = Instance.new("Frame")
card.Name = "Card"
card.Size = UDim2.fromOffset(WIDTH, HEIGHT)
card.BackgroundColor3 = Color3.fromRGB(22, 26, 36)
card.Parent = root
rounded(card, 14)
inset(card, 16, 14)
list(card, Enum.FillDirection.Vertical, 10)

label(card, "Title", 1, UDim2.new(1, 0, 0, 20), "Chat").TextSize = 16

--- THE PANE IS THE ONE THING WITH A WRITTEN WIDTH, and the Narrower and
--- Wider buttons change only that. It clips, so a row that cannot fit is cut
--- at the edge instead of drawn over the card.
local pane = Instance.new("Frame")
pane.Name = "Pane"
pane.LayoutOrder = 2
pane.Size = UDim2.fromOffset(PANE_MAX, 196)
pane.BackgroundTransparency = 1
pane.ClipsDescendants = true
pane.Parent = card
list(pane, Enum.FillDirection.Vertical, 8)

local body = Instance.new("Frame")
body.Name = "Body"
body.LayoutOrder = 1
body.Size = UDim2.new(1, 0, 0, 156)
body.BackgroundTransparency = 1
body.Parent = pane
list(body, Enum.FillDirection.Horizontal, 6)

local sidebar = Instance.new("Frame")
sidebar.Name = "Sidebar"
sidebar.LayoutOrder = 1
sidebar.Size = UDim2.fromScale(0.4, 1)
sidebar.BackgroundColor3 = Color3.fromRGB(30, 36, 50)
sidebar.Parent = body
rounded(sidebar, 10)
inset(sidebar, 10, 8)
list(sidebar, Enum.FillDirection.Vertical, 4)
bounds(sidebar, Vector2.zero, Vector2.new(140, math.huge))
for order, channel in { "# general", "# layout", "# random" } do
	local row = label(sidebar, channel, order, UDim2.new(1, 0, 0, 22), channel)
	row.TextColor3 = if order == 1 then TEXT else MUTED
end

local messages = Instance.new("Frame")
messages.Name = "Messages"
messages.LayoutOrder = 2
messages.Size = UDim2.new(0, 100, 1, 0)
messages.BackgroundColor3 = Color3.fromRGB(30, 36, 50)
messages.ClipsDescendants = true
messages.Parent = body
rounded(messages, 10)
inset(messages, 8, 8)
list(messages, Enum.FillDirection.Vertical, 6)
flex(messages, Enum.UIFlexMode.Grow)

local function bubble(order: number, from: string, text: string)
	local b = label(messages, "Bubble" .. order, order, UDim2.new(1, 0, 0, 26), `{from}: {text}`)
	b.BackgroundTransparency = 0
	b.BackgroundColor3 = Color3.fromRGB(44, 52, 72)
	b.TextSize = 12
	rounded(b, 8)
	inset(b, 10, 0)
	bounds(b, Vector2.zero, Vector2.new(240, math.huge))
end
for order, message in MESSAGES do
	bubble(order, message.from, message.text)
end

--- THE COMPOSER: a fixed + button, a field that takes the spare width, a
--- channel chip that gives width back first, and Send.
local composer = Instance.new("Frame")
composer.Name = "Composer"
composer.LayoutOrder = 2
composer.Size = UDim2.new(1, 0, 0, 32)
composer.BackgroundTransparency = 1
composer.Parent = pane
list(composer, Enum.FillDirection.Horizontal, 6).VerticalAlignment = Enum.VerticalAlignment.Center

local attach = label(composer, "Attach", 1, UDim2.fromOffset(32, 32), "+")
attach.BackgroundTransparency = 0
attach.BackgroundColor3 = Color3.fromRGB(44, 58, 84)
attach.TextXAlignment = Enum.TextXAlignment.Center
attach.TextSize = 18
rounded(attach, 8)

local field = label(composer, "Field", 2, UDim2.fromOffset(80, 32), "Message #general")
field.BackgroundTransparency = 0
field.BackgroundColor3 = Color3.fromRGB(36, 42, 58)
field.TextColor3 = MUTED
rounded(field, 8)
inset(field, 10, 0)
local fieldFlex = flex(field, Enum.UIFlexMode.Grow)

local chip = label(composer, "Chip", 3, UDim2.fromOffset(90, 24), "#general")
chip.BackgroundTransparency = 0
chip.BackgroundColor3 = ACCENT
chip.TextXAlignment = Enum.TextXAlignment.Center
chip.TextSize = 12
rounded(chip, 12)
flex(chip, Enum.UIFlexMode.Shrink)
bounds(chip, Vector2.new(64, 0), Vector2.new(math.huge, math.huge))

local send = Instance.new("TextButton")
send.Name = "Send"
send.LayoutOrder = 4
send.Size = UDim2.fromOffset(60, 32)
send.BackgroundColor3 = ACCENT
send.TextColor3 = TEXT
send.TextSize = 13
send.Text = "Send"
send.Parent = composer
rounded(send, 8)
--- CUSTOM WITH A GROWRATIO OF 0 takes none of the spare width; raising the
--- ratio is all it takes to make Send share it with the field.
local sendFlex = flex(send, Enum.UIFlexMode.Custom)
sendFlex.GrowRatio = 0

local settings = label(card, "Settings", 3, UDim2.new(1, 0, 0, 16), "")
settings.TextSize = 12
settings.TextColor3 = MUTED

local toolbar = Instance.new("Frame")
toolbar.Name = "Toolbar"
toolbar.LayoutOrder = 4
toolbar.Size = UDim2.new(1, 0, 0, 28)
toolbar.BackgroundTransparency = 1
toolbar.Parent = card
list(toolbar, Enum.FillDirection.Horizontal, 6)

local function control(name: string, order: number, text: string, onActivated: () -> ())
	local button = Instance.new("TextButton")
	button.Name = name
	button.LayoutOrder = order
	button.Size = UDim2.new(0, 0, 0, 28)
	button.AutomaticSize = Enum.AutomaticSize.X
	button.BackgroundColor3 = Color3.fromRGB(44, 58, 84)
	button.TextColor3 = TEXT
	button.TextSize = 12
	button.Text = text
	button.Parent = toolbar
	rounded(button, 8)
	inset(button, 12, 0)
	button.Activated:Connect(onActivated)
end

local function resize(by: number)
	local width = math.clamp(pane.Size.X.Offset + by, PANE_MIN, PANE_MAX)
	pane.Size = UDim2.fromOffset(width, pane.Size.Y.Offset)
end

control("Narrower", 1, "Narrower", function()
	resize(-PANE_STEP)
end)
control("Wider", 2, "Wider", function()
	resize(PANE_STEP)
end)
--- Grow, then Custom with a GrowRatio of 2, then None, then Grow again.
control("FieldMode", 3, "Field mode", function()
	if fieldFlex.FlexMode == Enum.UIFlexMode.Grow then
		fieldFlex.FlexMode = Enum.UIFlexMode.Custom
		fieldFlex.GrowRatio = 2
	elseif fieldFlex.FlexMode == Enum.UIFlexMode.Custom then
		fieldFlex.FlexMode = Enum.UIFlexMode.None
	else
		fieldFlex.FlexMode = Enum.UIFlexMode.Grow
	end
end)
control("SendRatio", 4, "Send ratio", function()
	sendFlex.GrowRatio = (sendFlex.GrowRatio + 1) % 3
end)

send.Activated:Connect(function()
	bubble(#messages:GetChildren(), "you", "sent from the composer")
end)

--- ONE PLACE WRITES THE SETTINGS LINE, and it listens to what the buttons
--- change rather than to the buttons.
local function describe()
	local mode = fieldFlex.FlexMode.Name
	if fieldFlex.FlexMode == Enum.UIFlexMode.Custom then
		mode ..= ` (GrowRatio {fieldFlex.GrowRatio})`
	end
	settings.Text = `Pane {pane.Size.X.Offset}  |  field {mode}  |  Send GrowRatio {sendFlex.GrowRatio}`
end
pane:GetPropertyChangedSignal("Size"):Connect(describe)
fieldFlex.Changed:Connect(describe)
sendFlex.Changed:Connect(describe)
describe()

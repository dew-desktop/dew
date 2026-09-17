--!strict
--[[
	Nameplate -- an applet written against the DataModel with no framework.

	    cargo run -- examples/widgets/nameplate

	`Instance.new`, property assignment, `Parent`, and the vocabulary. Nothing is
	imported, and every line would build the same tree on any host implementing
	the DataModel.

	What it shows, beyond drawing:

	  - reading a capability, and reading the ABSENCE of one
	  - methods on an instance: `GetChildren`, `IsA`, `GetPropertyChangedSignal`
	  - events: `Activated` on a button, `MouseEnter` and `MouseLeave` on a frame
	  - an image resolved through `mod://`, which reads beside this file
	  - `ZIndex` deciding what draws on top

	Click the button. The label counts up and the dot alternates colour, so a
	snapshot before and after differ.
]]

--- `dew` is a global carrying exactly the capabilities `dew.toml` granted.
local float = assert(dew.Float, "dew.toml must grant `float`")

local root = float({
	width = 340,
	height = 148,
	anchor = "topRight",
	offset = { x = 24, y = 24 },
})

--- Reading a capability, and reading the ABSENCE of one.
---
--- `dew.Storage` is present because `dew.toml` asked for `storage`.
--- `dew.Clipboard` is nil because it did not: not guarded, not refused at the
--- call, absent. That is the capability model in one comparison, and it is worth
--- a line on screen because it is the half of the contract a render cannot show.
local function capabilityReport(): string
	if dew.Storage == nil then
		return "storage: not granted"
	end
	dew.Storage.set("nameplate.mounted", tostring(dew.Time.now()))
	local seen = dew.Storage.get("nameplate.mounted") ~= nil
	local clipboard = if dew.Clipboard == nil then "clipboard absent" else "clipboard granted"
	return `storage: {if seen then "readable" else "wrote nothing"} - {clipboard}`
end

local function label(parent: Instance, name: string, y: number, size: number, colour: Color3): TextLabel
	local text = Instance.new("TextLabel")
	text.Name = name
	text.Size = UDim2.new(1, -36, 0, size + 8)
	text.Position = UDim2.new(0, 18, 0, y)
	text.BackgroundTransparency = 1
	text.TextSize = size
	text.TextColor3 = colour
	text.TextXAlignment = Enum.TextXAlignment.Left
	text.Parent = parent
	return text
end

local card = Instance.new("Frame")
card.Name = "Card"
card.Size = UDim2.new(1, 0, 1, 0)
card.BackgroundColor3 = Color3.fromRGB(20, 26, 36)
card.Parent = root

local corner = Instance.new("UICorner")
corner.CornerRadius = UDim.new(0, 14)
corner.Parent = card

local stroke = Instance.new("UIStroke")
stroke.Color = Color3.fromRGB(64, 110, 184)
stroke.Thickness = 1
stroke.Parent = card

label(card, "Title", 14, 22, Color3.fromRGB(233, 239, 249)).Text = "Nameplate"
label(card, "Subtitle", 42, 14, Color3.fromRGB(146, 162, 188)).Text = "an applet with no framework"
label(card, "Caps", 66, 12, Color3.fromRGB(120, 170, 240)).Text = capabilityReport()

--- `card:IsA("GuiObject")` is the one worth printing: `card` is a Frame, and the
--- answer is true only because `IsA` walks the superclass chain rather than
--- comparing strings.
label(card, "Runtime", 84, 12, Color3.fromRGB(104, 118, 142)).Text =
	`in {root.Name} - {#card:GetChildren()} kids - IsA GuiObject: {card:IsA("GuiObject")}`

--- A signal, connected and fired and disconnected.
---
--- Two assignments and only one of them is a change: the first writes the colour
--- the card already has, and nothing fires for it. Assigning the same value is
--- not an event, which is also how the host tells a static tree from a moving one
--- and skips the repaint.
local changes = 0
local watch = card:GetPropertyChangedSignal("BackgroundColor3"):Connect(function()
	changes += 1
end)
card.BackgroundColor3 = Color3.fromRGB(20, 26, 36)
card.BackgroundColor3 = Color3.fromRGB(23, 30, 41)
watch:Disconnect()

label(card, "Signals", 102, 12, Color3.fromRGB(104, 118, 142)).Text =
	`2 writes - {changes} Changed - {typeof(watch)} connected: {watch.Connected}`

--- `ZIndex` over declaration order. Drawn last on purpose: it overlaps the text.
local dot = Instance.new("Frame")
dot.Name = "Dot"
dot.Size = UDim2.new(0, 10, 0, 10)
dot.Position = UDim2.new(1, -18, 0, 22)
dot.AnchorPoint = Vector2.new(1, 0)
dot.BackgroundColor3 = Color3.fromRGB(96, 208, 148)
dot.ZIndex = 5
dot.Parent = card

local dotCorner = Instance.new("UICorner")
dotCorner.CornerRadius = UDim.new(1, 0)
dotCorner.Parent = dot

--- `mod://` RESOLVES BESIDE THIS FILE. It is the only scheme this host resolves:
--- an unresolvable `Content` draws as a missing box and says so by name, rather
--- than failing as a property error.
---
--- `ImageContent` is the modern property and `Image` is the legacy `ContentId`
--- beside it; both name the same asset and either works. `ScaleType.Fit` keeps
--- the aspect ratio.
local droplet = Instance.new("ImageLabel")
droplet.Name = "Droplet"
droplet.Size = UDim2.new(0, 44, 0, 44)
droplet.Position = UDim2.new(1, -18, 0, 46)
droplet.AnchorPoint = Vector2.new(1, 0)
droplet.BackgroundTransparency = 1
droplet.ImageContent = Content.fromUri("mod://droplet.png")
droplet.ScaleType = Enum.ScaleType.Fit
droplet.ZIndex = 4
droplet.Parent = card

--- A `TextButton` rather than a `Frame`, and the class is the decision:
--- `Activated` is a `GuiButton` event, so `frame.Activated` errors with "not a
--- valid member of Frame". A button is also `Active = true` by default, which is
--- what makes it a hit candidate without setting anything.
local button = Instance.new("TextButton")
button.Name = "Bump"
button.Size = UDim2.new(1, -36, 0, 24)
button.Position = UDim2.new(0, 18, 0, 120)
button.BackgroundColor3 = Color3.fromRGB(38, 52, 74)
button.TextColor3 = Color3.fromRGB(206, 222, 246)
button.TextSize = 12
button.Text = "click me"
button.Parent = card

local buttonCorner = Instance.new("UICorner")
buttonCorner.CornerRadius = UDim.new(0, 8)
buttonCorner.Parent = button

local clicks = 0
button.Activated:Connect(function()
	clicks += 1
	button.Text = `clicked {clicks}x`
	--- Alternating rather than setting one colour, because assigning the same
	--- value fires nothing and repaints nothing.
	dot.BackgroundColor3 = if clicks % 2 == 1
		then Color3.fromRGB(240, 186, 96)
		else Color3.fromRGB(96, 208, 148)
	--- `ImageColor3` multiplies through the asset's own pixels rather than
	--- replacing them: the droplet's white highlight takes the tint and its
	--- transparent corners stay transparent.
	droplet.ImageColor3 = if clicks % 2 == 1
		then Color3.fromRGB(240, 186, 96)
		else Color3.fromRGB(255, 255, 255)
end)

--- Hover on a frame needs one line of consent. A `Frame` ships `Active = false`
--- and is therefore not a hit candidate, which is the same default that stops a
--- `TextLabel` stealing a click from the button underneath it.
card.Active = true
card.MouseEnter:Connect(function()
	stroke.Color = Color3.fromRGB(120, 170, 240)
end)
card.MouseLeave:Connect(function()
	stroke.Color = Color3.fromRGB(64, 110, 184)
end)

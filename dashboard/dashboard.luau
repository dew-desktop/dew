--!strict
--[[
	The Dew dashboard -- browse the marketplace and install a package,
	without a terminal (milestone 23). `manage.rs`'s Win32 window still owns
	sign-in and the installed-applet list; this is the first native surface
	built on `dew.Marketplace` (ADR-017/ADR-018), not a replacement for it.

	Bundled with the host rather than installed like a marketplace package
	-- `dew.toml` asks for `discover` and `install`, `Capability::Host`
	permissions no ordinary applet can ever be granted.
]]

local vide = require("./roblox_packages/vide")
local source = vide.source
local effect = vide.effect

local window = assert(desktop.Window, "dew.toml must grant `window`")
local marketplace = assert(dew.Marketplace, "dew.toml must grant `discover` and `install`")

local root = window({ title = "Dew Dashboard", width = 420, height = 520 })

type Package = { ownerUserId: string, appletId: string, uploadedAt: string }

-- STATE FIRST, EVERYTHING ELSE AFTER. `buildRow` below closes over
-- `awaitingInstall`; declaring it after `buildRow` would make that a
-- capture of a global instead of this local, since a Luau chunk resolves
-- an upvalue at the point a function is DEFINED, not where it is called.
local packages = source(nil :: { Package }?)

-- AWAITING FLAGS, NOT A COMPARISON AGAINST THE LAST POLLED TABLE.
-- `Discovered()`/`Installed()` build a fresh Lua table every call (see
-- `capabilities.rs`), so two reads of an unchanged answer are never `==`
-- to each other -- polling every frame and writing straight into
-- `packages` would rebuild the whole row list every frame, forever, once
-- one result ever landed. A flag set at trigger time and cleared the one
-- time its answer is actually consumed is what keeps each result read
-- exactly once.
local awaitingDiscover = false
local awaitingInstall = false

local function label(
	parent: Instance,
	name: string,
	position: UDim2,
	size: UDim2,
	textSize: number,
	colour: Color3
): TextLabel
	local text = Instance.new("TextLabel")
	text.Name = name
	text.Position = position
	text.Size = size
	text.BackgroundTransparency = 1
	text.TextSize = textSize
	text.TextColor3 = colour
	text.TextXAlignment = Enum.TextXAlignment.Left
	text.Parent = parent
	return text
end

local body = Instance.new("Frame")
body.Name = "Body"
body.Size = UDim2.new(1, 0, 1, 0)
body.BackgroundColor3 = Color3.fromRGB(18, 20, 26)
body.Parent = root

label(body, "Title", UDim2.new(0, 16, 0, 12), UDim2.new(1, -112, 0, 24), 18, Color3.fromRGB(233, 239, 249)).Text =
	"Marketplace"

local refreshButton = Instance.new("TextButton")
refreshButton.Name = "Refresh"
refreshButton.Position = UDim2.new(1, -96, 0, 12)
refreshButton.Size = UDim2.new(0, 80, 0, 24)
refreshButton.BackgroundColor3 = Color3.fromRGB(38, 52, 74)
refreshButton.TextColor3 = Color3.fromRGB(206, 222, 246)
refreshButton.TextSize = 12
refreshButton.Text = "Refresh"
refreshButton.Parent = body

-- WRAPPED, AND GIVEN ROOM FOR MORE THAN ONE LINE: a network or auth error
-- (see `platform::describe_auth_error`) is a full sentence, not a label,
-- and an unwrapped one just runs off the edge of the window rather than
-- being clipped -- which reads as "blank" at a glance, not as an error.
local status = label(body, "Status", UDim2.new(0, 16, 0, 44), UDim2.new(1, -32, 0, 40), 12, Color3.fromRGB(146, 162, 188))
status.TextWrapped = true
status.TextYAlignment = Enum.TextYAlignment.Top

local list = Instance.new("ScrollingFrame")
list.Name = "List"
list.Position = UDim2.new(0, 16, 0, 92)
list.Size = UDim2.new(1, -32, 1, -108)
list.BackgroundTransparency = 1
-- A ROW IS SIZED AGAINST THE CANVAS, NOT THE FRAME'S OWN VISIBLE BOX --
-- `render.rs` lays every child out inside `CanvasSize`'s box, and that
-- defaults to zero on both axes. Without this, `UDim2.new(1, 0, 0, ...)`
-- on a row resolves its width against a zero-wide canvas: every row
-- exists, at zero width, which is indistinguishable from nothing at all.
-- Width tracks the frame itself; height grows to fit whatever the list
-- layout below actually lays out.
list.CanvasSize = UDim2.new(1, 0, 0, 0)
list.AutomaticCanvasSize = Enum.AutomaticSize.Y
list.Parent = body

local listLayout = Instance.new("UIListLayout")
listLayout.Padding = UDim.new(0, 8)
listLayout.Parent = list

local ROW_HEIGHT = 64

local function clearRows()
	for _, child in ipairs(list:GetChildren()) do
		if child:IsA("Frame") and child.Name == "Row" then
			child:Destroy()
		end
	end
end

local function buildRow(pkg: Package)
	local row = Instance.new("Frame")
	row.Name = "Row"
	row.Size = UDim2.new(1, 0, 0, ROW_HEIGHT)
	row.BackgroundColor3 = Color3.fromRGB(28, 31, 40)
	row.Parent = list

	local corner = Instance.new("UICorner")
	corner.CornerRadius = UDim.new(0, 8)
	corner.Parent = row

	label(row, "Name", UDim2.new(0, 12, 0, 8), UDim2.new(1, -100, 0, 18), 14, Color3.fromRGB(233, 239, 249)).Text =
		`@{pkg.ownerUserId}/{pkg.appletId}`
	label(row, "Meta", UDim2.new(0, 12, 0, 30), UDim2.new(1, -100, 0, 16), 11, Color3.fromRGB(146, 162, 188)).Text =
		`published {pkg.uploadedAt}`

	local install = Instance.new("TextButton")
	install.Name = "Install"
	install.Position = UDim2.new(1, -84, 0.5, -12)
	install.Size = UDim2.new(0, 72, 0, 24)
	install.BackgroundColor3 = Color3.fromRGB(48, 90, 64)
	install.TextColor3 = Color3.fromRGB(214, 240, 224)
	install.TextSize = 12
	install.Text = "Install"
	install.Parent = row

	install.Activated:Connect(function()
		status.Text = `Installing {pkg.appletId}...`
		awaitingInstall = true
		marketplace.Install(pkg.ownerUserId, pkg.appletId)
	end)
end

-- THE SOURCE IS THE ONLY STATE; THE EFFECT IS THE ONLY PLACE THAT REACTS,
-- the same split `widget-behaviors` already uses. `desktop.Clock.OnFrame`
-- below only senses and pushes; rebuilding the row list is this effect's
-- job alone.
vide.root(function()
	effect(function()
		local current = packages()
		clearRows()
		if current == nil then
			return
		end
		for _, pkg in ipairs(current) do
			buildRow(pkg)
		end
	end)
end)

desktop.Clock.OnFrame(function()
	if awaitingDiscover then
		local discovered = marketplace.Discovered()
		if discovered ~= nil then
			awaitingDiscover = false
			if discovered.ok then
				packages(discovered.packages)
				status.Text = if #discovered.packages == 0 then "No public packages yet" else ""
			else
				status.Text = discovered.error
			end
		end
	end

	if awaitingInstall then
		local installed = marketplace.Installed()
		if installed ~= nil then
			awaitingInstall = false
			status.Text = if installed.ok then `Installed '{installed.id}'` else installed.error
		end
	end
end)

refreshButton.Activated:Connect(function()
	status.Text = "Refreshing..."
	awaitingDiscover = true
	marketplace.Discover()
end)

status.Text = "Loading..."
awaitingDiscover = true
marketplace.Discover()

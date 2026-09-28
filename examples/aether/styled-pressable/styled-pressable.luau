--!strict
--[[
	A Pressable whose corner roundness comes entirely from a real
	StyleSheet cascade.

	MILESTONE 28 SPRINT 4's OWN QUESTION: does Aether's existing
	architecture already compose with a real StyleSheet/StyleRule/
	StyleDerive cascade, or does it need new code first? Partly -- and
	the "partly" is itself the real finding.

	FIRST ATTEMPT WAS `BackgroundColor3` ON THE BUTTON ITSELF, AND IT DID
	NOT WORK, live-tested and diagnosed rather than assumed: vide's own
	`create.luau` (not Aether's `createPressable.luau`, which sets no
	such default) writes `BackgroundColor3`/`BorderColor3`/
	`BorderSizePixel` onto every `Frame` it builds, from its own
	`defaults.luau` table, BEFORE a caller's props are applied. That
	write is a real, explicit instance value the moment `create()`
	returns -- `Dom::styled_property`'s own contract ("an instance's own
	explicit value first") correctly refuses to let a `StyleRule`
	override it, which is right for ordinary use and is exactly what
	blocked this demo's first version. This is a genuine gap between
	vide's own default-property convention and the real cascade's
	explicit-wins semantics, not a bug in either one alone -- named here,
	not routed around silently, and left for its own follow-up rather
	than "fixed" by weakening `styled_property`'s own correct rule.

	SECOND ATTEMPT, BELOW, THEMES `UICorner.CornerRadius` INSTEAD, and it
	works: `UICorner` has no entry in vide's own `defaults.luau` at all,
	so nothing pre-empts the cascade for it. The button's own
	`BackgroundColor3` stays a plain literal, undermined, proving the
	real, narrower claim this sprint can actually stand behind: a real
	Aether-built primitive's OWN CHILD, styled through `StyleDerive` and
	a `$Token`, swaps live with no restart.
]]

local Aether = require("./roblox_packages/aether")
local vide = require("./roblox_packages/vide")

local WIDTH, HEIGHT = 240, 140

local root = desktop.Widget({ width = WIDTH, height = HEIGHT, anchor = "center" })

local CollectionService = (services :: any):GetService("CollectionService")

-- `Aether.Pressable` calls `vide.cleanup` internally, which needs a real
-- reactive scope to register against -- everything below runs inside one.
vide.root(function()
	local label = Instance.new("TextLabel")
	label.Size = UDim2.new(1, 0, 0, 20)
	label.BackgroundTransparency = 1
	label.TextColor3 = Color3.fromRGB(220, 220, 220)
	label.Text = "theme: sharp"
	label.Parent = root

	-- THE TWO THEMES. Each is a plain StyleSheet carrying one token and
	-- nothing else -- no StyleRule of its own, no selector, just an
	-- Attribute a StyleDerive elsewhere can compose from.
	local sharp = Instance.new("StyleSheet")
	sharp.Name = "Sharp"
	sharp:SetAttribute("Radius", UDim.new(0, 2))
	sharp.Parent = root

	local round = Instance.new("StyleSheet")
	round.Name = "Round"
	round:SetAttribute("Radius", UDim.new(0, 28))
	round.Parent = root

	-- THE DESIGN SHEET. This is the one that actually reaches the
	-- button's own UICorner child (parented on `root`, their common
	-- ancestor), and the only one with a real StyleRule.
	local design = Instance.new("StyleSheet")
	design.Name = "Design"
	design.Parent = root

	local derive = Instance.new("StyleDerive")
	derive.Parent = design
	derive.StyleSheet = sharp

	local rule = Instance.new("StyleRule")
	rule.Selector = ".ThemedCorner"
	rule:SetProperty("CornerRadius", "$Radius")
	rule.Parent = design

	-- THE UICORNER, tagged so the rule above can reach it. Its own
	-- `CornerRadius` is never set here -- the cascade owns it alone.
	local corner = Aether.create("UICorner")({})
	CollectionService:AddTag(corner, "ThemedCorner")

	Aether.Pressable({
		Parent = root,
		Name = "Button",
		Position = UDim2.new(0, 20, 0, 30),
		Size = UDim2.new(0, 100, 0, 70),
		BackgroundTransparency = 0,
		BackgroundColor3 = Color3.fromRGB(70, 110, 200),

		corner,
	})

	local onSharp = true

	Aether.Pressable({
		Parent = root,
		Name = "SwapTheme",
		Position = UDim2.new(0, 140, 0, 30),
		Size = UDim2.new(0, 80, 0, 70),
		BackgroundTransparency = 0,
		BackgroundColor3 = Color3.fromRGB(60, 60, 68),

		OnActivated = function()
			onSharp = not onSharp
			derive.StyleSheet = if onSharp then sharp else round
			label.Text = if onSharp then "theme: sharp" else "theme: round"
		end,

		Aether.create("TextLabel")({
			Size = UDim2.fromScale(1, 1),
			BackgroundTransparency = 1,
			TextColor3 = Color3.fromRGB(230, 230, 230),
			TextSize = 12,
			Text = "swap theme",
		}),
	})
end)

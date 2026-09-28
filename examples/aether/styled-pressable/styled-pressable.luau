--!strict
--[[
	A Pressable colored AND shaped entirely through a real StyleSheet
	cascade.

	THE FULL STORY, IN TWO PARTS.

	First attempt themed `BackgroundColor3` directly and it did not work,
	live-tested and diagnosed rather than assumed: `vide`'s own
	`create.luau` (not Aether's `createPressable.luau`, which sets no
	such default) wrote `BackgroundColor3`/`BorderColor3`/
	`BorderSizePixel` onto every `Frame` it built, from its own
	`defaults.luau` table, before a caller's props applied. That write
	was a real, explicit instance value the moment `create()` returned --
	`Dom::styled_property`'s own contract ("an instance's own explicit
	value first") correctly refused to let a `StyleRule` override it,
	which was right for ordinary use and is exactly what blocked the
	first version of this demo. Fixed upstream, in `vide` itself
	(`es9-dev/vide`, `flags.luau`): the write is skipped under Dew
	specifically, since Dew's own DataModel already returns the identical
	default from the same reflection data `defaults.luau` was hand-copied
	from, so the write added nothing there and only got in a `StyleRule`'s
	way.

	Below, both `BackgroundColor3` (the button itself, tagged) and
	`UICorner.CornerRadius` (its own child, tagged separately) are themed
	through the SAME `StyleDerive`, proving the fix rather than merely
	asserting it: color theming needed the `vide` fix; corner theming
	never did (`UICorner` has no entry in `vide`'s own defaults table at
	all) and worked from this example's very first version.
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
	label.Text = "theme: sunrise"
	label.Parent = root

	-- THE TWO THEMES. Each is a plain StyleSheet carrying two tokens and
	-- nothing else -- no StyleRule of its own, no selector, just
	-- Attributes a StyleDerive elsewhere can compose from.
	local sunrise = Instance.new("StyleSheet")
	sunrise.Name = "Sunrise"
	sunrise:SetAttribute("AccentColor", Color3.fromRGB(240, 140, 40))
	sunrise:SetAttribute("Radius", UDim.new(0, 2))
	sunrise.Parent = root

	local dusk = Instance.new("StyleSheet")
	dusk.Name = "Dusk"
	dusk:SetAttribute("AccentColor", Color3.fromRGB(90, 70, 200))
	dusk:SetAttribute("Radius", UDim.new(0, 28))
	dusk.Parent = root

	-- THE DESIGN SHEET. This is the one that actually reaches the button
	-- and its own UICorner child (parented on `root`, their common
	-- ancestor), and the only one with real StyleRules.
	local design = Instance.new("StyleSheet")
	design.Name = "Design"
	design.Parent = root

	local derive = Instance.new("StyleDerive")
	derive.Parent = design
	derive.StyleSheet = sunrise

	local colorRule = Instance.new("StyleRule")
	colorRule.Selector = ".ThemedButton"
	colorRule:SetProperty("BackgroundColor3", "$AccentColor")
	colorRule.Parent = design

	local cornerRule = Instance.new("StyleRule")
	cornerRule.Selector = ".ThemedCorner"
	cornerRule:SetProperty("CornerRadius", "$Radius")
	cornerRule.Parent = design

	-- THE UICORNER, tagged so `cornerRule` can reach it. Its own
	-- `CornerRadius` is never set here -- the cascade owns it alone.
	local corner = Aether.create("UICorner")({})
	CollectionService:AddTag(corner, "ThemedCorner")

	-- THE BUTTON ITSELF. Neither `BackgroundColor3` NOR
	-- `BackgroundTransparency` beyond the one literal needed to make it
	-- opaque is themed here in Luau -- `BackgroundColor3` is never named
	-- at all, on purpose, so the cascade owns it alone.
	local button = Aether.Pressable({
		Parent = root,
		Name = "Button",
		Position = UDim2.new(0, 20, 0, 30),
		Size = UDim2.new(0, 100, 0, 70),
		BackgroundTransparency = 0,

		corner,
	})
	CollectionService:AddTag(button, "ThemedButton")

	local onSunrise = true

	Aether.Pressable({
		Parent = root,
		Name = "SwapTheme",
		Position = UDim2.new(0, 140, 0, 30),
		Size = UDim2.new(0, 80, 0, 70),
		BackgroundTransparency = 0,
		BackgroundColor3 = Color3.fromRGB(60, 60, 68),

		OnActivated = function()
			onSunrise = not onSunrise
			derive.StyleSheet = if onSunrise then sunrise else dusk
			label.Text = if onSunrise then "theme: sunrise" else "theme: dusk"
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

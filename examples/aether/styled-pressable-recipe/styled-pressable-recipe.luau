--!strict
--[[
	The exact same scenario as `../styled-pressable/styled-pressable.luau`
	-- a Pressable's `BackgroundColor3` and its own `UICorner`'s
	`CornerRadius`, both themed live across two `StyleSheet`s -- but
	authored through `createRecipe`/`recipe.Bind`/`recipe.SetThemeSource`
	(milestone 60 Part A) instead of by hand with
	`Instance.new("StyleRule")`/`("StyleDerive")` and a caller-managed
	`derive.StyleSheet = ...` re-point.

	WHAT STAYS THE SAME, ON PURPOSE. The two theme sheets (`sunrise`/
	`dusk`) are still plain `StyleSheet`s carrying only `Attribute`s --
	that half of the hand-built pattern was already correct, and
	`createRecipe`'s own header says as much: it hands token strings
	through unresolved, it does not invent a second resolution path.

	WHAT CHANGES. The button's own styling is one `recipe({ base = ... })`
	definition (no variants needed -- this scenario has exactly one look,
	themed by re-pointing the source, not switched between several), bound
	to the button and to its `UICorner` child with `recipe.Bind`, both
	with the SAME (empty) selection -- so both instances carry the SAME
	generated tag and share the ONE `StyleRule` `resolve` produces, which
	carries both `BackgroundColor3` and `CornerRadius`. `resolve_full`'s
	own doc comment (`cascade.rs` line 38: "not filtered against the
	target's own declared properties") is why binding a Frame and a
	UICorner to one two-property rule is safe -- a paint pass reads only
	the property name it asks for, per instance type, and this cascade
	does not know or care which names a given instance actually has.
	Theme swapping goes through `recipe.SetThemeSource`, never touching
	`derive.StyleSheet` directly.
]]

local Aether = require("./roblox_packages/aether")
local vide = require("./roblox_packages/vide")
local newDom = require("./dom")

local WIDTH, HEIGHT = 240, 140

local root = desktop.Widget({ width = WIDTH, height = HEIGHT, anchor = "center" })

-- THE DOM PORT. Closed over `root` so a `StyleSheet` it creates can be
-- parented there right away -- see `dom.luau`'s own header for why that
-- parenting has to happen somewhere, and why it is this adapter's call
-- to make rather than `createRecipe.luau`'s.
local dom = newDom(root)

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
	-- Attributes the recipe layer's own shared StyleDerive composes from.
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

	-- THE RECIPE. One definition, one property table, no variants -- the
	-- "Aether recipe" StyleSheet it resolves into is parented on `root`
	-- by the recipe layer itself the first time `resolve`/`Bind` runs.
	local recipe = Aether.Core.createRecipe({
		base = {
			BackgroundColor3 = "$AccentColor",
			CornerRadius = "$Radius",
		},
	}, { Dom = dom, Component = "Pressable" })

	-- THE UICORNER, bound to the recipe's own (single) resolved
	-- combination. Its own `CornerRadius` is never set here -- the
	-- cascade owns it alone.
	local corner = Aether.create("UICorner")({})
	recipe.Bind(corner, {})

	-- THE BUTTON ITSELF, bound to the same recipe and the same empty
	-- selection -- same tag, same shared StyleRule as `corner` above.
	-- Neither `BackgroundColor3` nor `BackgroundTransparency` beyond the
	-- one literal needed to make it opaque is themed here in Luau.
	local button = Aether.Pressable({
		Parent = root,
		Name = "Button",
		Position = UDim2.new(0, 20, 0, 30),
		Size = UDim2.new(0, 100, 0, 70),
		BackgroundTransparency = 0,

		corner,
	})
	recipe.Bind(button, {})

	-- THE INITIAL THEME. Matches `styled-pressable.luau`'s own eager
	-- `derive.StyleSheet = sunrise` -- without this call the shared
	-- StyleDerive `SetThemeSource` creates on first use would not exist
	-- yet, and `$AccentColor`/`$Radius` would resolve to nothing.
	local onSunrise = true
	recipe.SetThemeSource(sunrise)

	Aether.Pressable({
		Parent = root,
		Name = "SwapTheme",
		Position = UDim2.new(0, 140, 0, 30),
		Size = UDim2.new(0, 80, 0, 70),
		BackgroundTransparency = 0,
		BackgroundColor3 = Color3.fromRGB(60, 60, 68),

		OnActivated = function()
			onSunrise = not onSunrise
			recipe.SetThemeSource(if onSunrise then sunrise else dusk)
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

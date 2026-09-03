--!strict
--[[
	Nameplate — a Dew mod written against no framework at all.

	    cargo run --manifest-path host/Cargo.toml -- --mod nameplate --snapshot out.png

	THE THIRD MOD, AND THE FIRST THAT IMPORTS NOTHING. `calculator`, `timetracker`
	and `workspace` are Aether components: they `require("@aether/api")`, return a
	tree the framework owns, and the host drives them through a `Session`. This
	file has no `require` line. It builds instances, sets properties, parents them
	into the root it was handed, and returns — which is exactly what the same code
	would do inside Roblox.

	IT IS A MOD, NOT A SCRIPT, and that is the whole point of this sprint.
	`examples/standalone/app.luau` already proved the DataModel reaches a pixel,
	but it was run with `--script`: no `mod.json`, no capability table, no surface
	placement, and not discovered from `mods/`. This one arrives through the
	ordinary loader with all four.

	`runtime = "datamodel"` IN `mod.json` IS WHAT SELECTS THIS PATH. The host does
	not sniff for a framework — declaring it is the author's job, because both
	flavours build a tree and there is no artefact that tells them apart.

	WHAT IT DOES NOT DO, and says so rather than faking: nothing here reacts.
	There is no signal model behind a DataModel tree yet, so `mount` runs once and
	what it built is what stays on screen. The host re-renders every frame anyway,
	which means the day `Changed` exists (sprint 8) an animating mod works with no
	change to this file.
]]

--- Reading a capability, and reading the ABSENCE of one.
---
--- `dew.storage` is present because `mod.json` asked for `storage` and the host
--- granted it. `dew.clipboard` is nil because it did not ask — not guarded, not
--- refused at the call, ABSENT. That is the capability model in one comparison,
--- and it is worth a line on screen because it is the half of the mod contract a
--- render normally cannot show.
local function capabilityReport(dew: any): string
	if dew.storage == nil then
		return "storage: not granted"
	end
	dew.storage.set("nameplate.mounted", tostring(dew.time.now()))
	local seen = dew.storage.get("nameplate.mounted") ~= nil
	local clipboard = if dew.clipboard == nil then "clipboard absent" else "clipboard granted"
	return `storage: {if seen then "readable" else "wrote nothing"} · {clipboard}`
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

return {
	id = "nameplate",
	size = { width = 340, height = 148 },

	--- A widget, top-right, like the platform's default. Declared rather than
	--- inherited so this file shows the surface block a mod author writes.
	surface = {
		kind = "widget",
		anchor = "topRight",
		offset = { x = 24, y = 24 },
	},

	--- `mount(dew, root)`. Two arguments, and neither is a global: the capability
	--- table because what a mod may DO is per mod, and the root because what a mod
	--- may DRAW INTO is the surface the host opened for it. An Aether mod's
	--- `mount(dew)` returns a tree instead; that difference is the runtime.
	mount = function(dew: any, root: Instance)
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

		label(card, "Title", 16, 22, Color3.fromRGB(233, 239, 249)).Text = "Nameplate"
		label(card, "Subtitle", 48, 14, Color3.fromRGB(146, 162, 188)).Text =
			"a mod with no framework"
		label(card, "Caps", 76, 12, Color3.fromRGB(120, 170, 240)).Text = capabilityReport(dew)
		--- THREE METHOD CALLS, and until sprint 7 this line could not be written.
		--- The member surface read 0 of 52 when this file was first committed, and
		--- the comment here said so; tree navigation and lifecycle are the first
		--- 14 of it. A test calling `GetChildren` on a bare VM proves the dispatch
		--- works. This proves it survives the ordinary loader — the same VM a mod
		--- is really given, with the same globals, discovered from `mods/`.
		---
		--- `card:IsA("GuiObject")` is the one worth printing: `card` is a Frame,
		--- and the answer is true only because `IsA` walks the superclass chain
		--- rather than comparing strings.
		label(card, "Runtime", 100, 12, Color3.fromRGB(104, 118, 142)).Text =
			`in {root.Name} · {#card:GetChildren()} kids · IsA GuiObject: {card:IsA("GuiObject")}`

		--- ZIndex over declaration order, the one rule a geometric conformance
		--- runner cannot check. Drawn last on purpose: it overlaps the text.
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
	end,
}

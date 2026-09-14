--!strict
--[[
	Nameplate — a Dew applet written against no framework at all.

	    cargo run --manifest-path host/Cargo.toml -- snapshot applets/nameplate -o out.png

	THE THIRD MOD, AND THE FIRST THAT IMPORTS NOTHING. `calculator`, `timetracker`
	and `workspace` are Aether components: they `require("./roblox_packages/aether")`, return a
	tree the framework owns, and the host drives them through a `Session`. This
	file has no `require` line. It builds instances, sets properties, parents them
	into the root it was handed, and returns — which is exactly what the same code
	would do inside the engine.

	IT IS A MOD, NOT A SCRIPT, and that is the whole point of this sprint.
	`examples/standalone/app.luau` already proved the DataModel reaches a pixel,
	but it was run with `--script`: no `dew.toml`, no capability table, no surface
	placement, and not discovered from `applets/`. This one arrives through the
	ordinary loader with all four.

	`runtime = "datamodel"` IN `dew.toml` IS WHAT SELECTS THIS PATH. The host does
	not sniff for a framework — declaring it is the author's job, because both
	flavours build a tree and there is no artefact that tells them apart.

	AND IT IS CLICKABLE NOW, which is the clause milestone 1's done-test has been
	missing since it was written. The button at the bottom of the card is a real
	`TextButton` with a real `Activated` connection: clicking it counts up, recolours
	the dot, and rewrites its own label. Sprint 7 recorded that pointer events were
	dropped for a DataModel mod and named the cause — fourteen methods and no events.
	Sprint 8 built the signal type; sprint 9 added the hit test that says which
	instance is at (x, y), and this is it being clicked.

	HOVER TOO, on the card itself. `card.Active = true` is what makes a Frame a hit
	candidate: a Frame ships `Active = false`, and that is the engine's own answer to
	how input is gated rather than this host's invention — it is why a TextLabel laid
	over a button does not steal the click. The rule Dew honours is written down in
	`host/src/datamodel/input.rs`: `Active` decides who is hit, `Interactable` decides
	whether the hit fires anything.

	AND THE HOST NO LONGER REPAINTS THIS EVERY FRAME. It used to, because nothing
	signalled a change and "assume so" was the only honest answer; the same
	plumbing that fires `Changed` now marks the tree dirty, so a card that never
	moves is drawn once. An applet that animates does it by assigning to a property,
	which is exactly what dirties the tree — so that costs an animating mod
	nothing, and this file did not have to change for either.

	INPUT DOES NOT CHANGE THAT EITHER. Moving the cursor over this card fires
	`MouseEnter` and `MouseMoved`, and none of it repaints anything unless a handler
	actually assigns something. The dot recolouring on hover DOES repaint, once, per
	crossing — which is the point: `painted` rises while you interact and returns to
	zero when you stop.
]]

--- Reading a capability, and reading the ABSENCE of one.
---
--- `dew.storage` is present because `dew.toml` asked for `storage` and the host
--- granted it. `dew.clipboard` is nil because it did not ask — not guarded, not
--- refused at the call, ABSENT. That is the capability model in one comparison,
--- and it is worth a line on screen because it is the half of the applet contract a
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
	--- inherited so this file shows the surface block an applet author writes.
	surface = {
		kind = "widget",
		anchor = "topRight",
		offset = { x = 24, y = 24 },
	},

	--- `mount(dew, root)`. Two arguments, and neither is a global: the capability
	--- table because what an applet may DO is per mod, and the root because what a mod
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

		label(card, "Title", 14, 22, Color3.fromRGB(233, 239, 249)).Text = "Nameplate"
		label(card, "Subtitle", 42, 14, Color3.fromRGB(146, 162, 188)).Text =
			"a mod with no framework"
		label(card, "Caps", 66, 12, Color3.fromRGB(120, 170, 240)).Text = capabilityReport(dew)
		--- THREE METHOD CALLS, and until sprint 7 this line could not be written.
		--- The member surface read 0 of 52 when this file was first committed, and
		--- the comment here said so; tree navigation and lifecycle are the first
		--- 14 of it. A test calling `GetChildren` on a bare VM proves the dispatch
		--- works. This proves it survives the ordinary loader — the same VM a mod
		--- is really given, with the same globals, discovered from `applets/`.
		---
		--- `card:IsA("GuiObject")` is the one worth printing: `card` is a Frame,
		--- and the answer is true only because `IsA` walks the superclass chain
		--- rather than comparing strings.
		label(card, "Runtime", 84, 12, Color3.fromRGB(104, 118, 142)).Text =
			`in {root.Name} · {#card:GetChildren()} kids · IsA GuiObject: {card:IsA("GuiObject")}`

		--- A SIGNAL, CONNECTED AND FIRED AND DISCONNECTED, on the ordinary loader.
		---
		--- Two assignments and one of them is not a change: the first writes the
		--- colour the card already has, and the host fires nothing for it. That is
		--- the decision this sprint made about `Changed` — assigning the same
		--- value is not an event — and it is the reason the host can now tell a
		--- static tree from a moving one and skip the repaint. Printing the count
		--- puts the rule on screen instead of in a comment.
		local changes = 0
		local watch = card:GetPropertyChangedSignal("BackgroundColor3"):Connect(function()
			changes += 1
		end)
		card.BackgroundColor3 = Color3.fromRGB(20, 26, 36)
		card.BackgroundColor3 = Color3.fromRGB(23, 30, 41)
		watch:Disconnect()

		label(card, "Signals", 102, 12, Color3.fromRGB(104, 118, 142)).Text =
			`2 writes · {changes} Changed · {typeof(watch)} connected: {watch.Connected}`

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

		--- AN IMAGE, AND THE WHOLE POINT OF MILESTONE 2's SPRINT 4.
		---
		--- `ImageContent` IS THE MODERN PROPERTY and `Image` is the legacy
		--- `ContentId` beside it; both name the same asset and this host takes
		--- either. `Image = "mod://droplet.png"` renders identically — the render
		--- pass prefers `ImageContent` only when both are set, because that is the
		--- one the engine's own migration keeps.
		---
		--- `mod://` RESOLVES BESIDE THIS FILE, which is `applets/nameplate/`. It is
		--- the only scheme this host resolves today: `rbxassetid://` needs the
		--- scheme registry, the permission and the fetch that are Sprint 5, and
		--- until then it reports itself by name and draws as a missing box rather
		--- than as nothing. That is ADR-003's rule — an unresolvable `Content` is
		--- a rendering outcome, not a property error.
		---
		--- `ScaleType.Fit` KEEPS THE ASPECT RATIO. The asset is square and so is
		--- the box, so it changes nothing here and says which rule is in force;
		--- `Slice` and `Tile` are not drawn yet and say so out loud when asked for.
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

		--- A BUTTON, AND THE WHOLE POINT OF SPRINT 9.
		---
		--- A `TextButton` RATHER THAN A `Frame`, and the class is the decision:
		--- `Activated` is a `GuiButton` event and a Frame does not have it —
		--- `frame.Activated` errors with "not a valid member of Frame", because the
		--- host's member predicate walks the superclass chain rather than matching a
		--- name. It is also `Active = true` by default, which is what makes it a hit
		--- candidate without this file setting anything.
		local button = Instance.new("TextButton")
		button.Name = "Bump"
		--- THE ROWS ABOVE WERE COMPACTED TO MAKE ROOM, rather than the card being
		--- made taller. A taller window would have changed the raster cost and made
		--- this sprint's `--stats` numbers not comparable with sprint 7's and 8's,
		--- which are measured at 340x148 — and the whole reason for taking those
		--- numbers is the comparison.
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

		--- THE TREE CHANGES, WHICH IS THE OBSERVABLE PART. A click that only
		--- incremented a counter would prove the handler ran and nothing else; this
		--- rewrites the label and recolours the dot, so what happened is on screen
		--- and a `--snapshot` before and after differ.
		local clicks = 0
		button.Activated:Connect(function()
			clicks += 1
			button.Text = `clicked {clicks}x`
			--- ASSIGNING THE SAME VALUE FIRES NOTHING AND REPAINTS NOTHING, which is
			--- sprint 8's rule and the reason this alternates rather than setting one
			--- colour: every click has to be a real change to be worth a frame.
			dot.BackgroundColor3 = if clicks % 2 == 1
				then Color3.fromRGB(240, 186, 96)
				else Color3.fromRGB(96, 208, 148)
			--- AND THE IMAGE IS TINTED THE SAME WAY, which is `ImageColor3`
			--- multiplying through the asset's own pixels rather than replacing
			--- them: the droplet's white highlight takes the tint and its
			--- transparent corners stay transparent. Sitting in the click handler
			--- rather than in the mount is deliberate — a property that is only
			--- ever set once cannot show that it reaches the painter on a REPAINT.
			droplet.ImageColor3 = if clicks % 2 == 1
				then Color3.fromRGB(240, 186, 96)
				else Color3.fromRGB(255, 255, 255)
		end)

		--- HOVER, ON A FRAME, WHICH NEEDS ONE LINE OF CONSENT.
		---
		--- `card.Active = true` is that line. A `Frame` ships `Active = false` and is
		--- therefore not a hit candidate at all — the same default that stops a
		--- `TextLabel` stealing a click from the button underneath it. An applet that
		--- wants a hoverable panel says so, which is what an engine developer writes
		--- for one anyway.
		card.Active = true
		card.MouseEnter:Connect(function()
			stroke.Color = Color3.fromRGB(120, 170, 240)
		end)
		card.MouseLeave:Connect(function()
			stroke.Color = Color3.fromRGB(64, 110, 184)
		end)
	end,
}

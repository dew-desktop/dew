--!strict
--[[
	A Dew applet with no framework at all.

	    cargo run --manifest-path host/Cargo.toml -- \
	        --script examples/standalone/app.luau --size 360x220 --snapshot out.png

	EVERY LINE BELOW WOULD BUILD THE SAME TREE INSIDE ROBLOX. That is the whole
	claim of the DataModel Standard, and this file is the smallest thing that
	demonstrates it: `Instance.new`, property assignment, `Parent`, and the
	vocabulary. No `require`, no framework, nothing imported.

	`DewRoot` RATHER THAN `game`, and the difference is not cosmetic.
	`Host.detect()` in Aether keys on `typeof(game) == "Instance"`, so a `game`
	global would flip every Aether mod in the same binary onto the Roblox branch
	and break it. `game` arrives when the services and the member surface are
	behind it; until then a standalone script parents into `DewRoot`.

	WHAT THIS EXERCISES, end to end: the reflection database rejecting a class or
	property that does not exist, the vocabulary types, `Enum` resolved from that
	same database, the layout pass, and the rasteriser. If this renders, the
	DataModel path is alive from Luau to pixels.
]]

local card = Instance.new("Frame")
card.Name = "Card"
card.Size = UDim2.new(1, -24, 1, -24)
card.Position = UDim2.new(0.5, 0, 0.5, 0)
card.AnchorPoint = Vector2.new(0.5, 0.5)
card.BackgroundColor3 = Color3.fromRGB(24, 30, 42)
card.Parent = DewRoot

--- A modifier is a property of its parent expressed as a child. Roblox's shape,
--- and the renderer must not draw it as a rectangle.
local corner = Instance.new("UICorner")
corner.CornerRadius = UDim.new(0, 16)
corner.Parent = card

local stroke = Instance.new("UIStroke")
stroke.Color = Color3.fromRGB(70, 120, 200)
stroke.Thickness = 2
stroke.Parent = card

local title = Instance.new("TextLabel")
title.Name = "Title"
title.Size = UDim2.new(1, -32, 0, 34)
title.Position = UDim2.new(0, 16, 0, 14)
title.BackgroundTransparency = 1
title.Text = "Dew standalone"
title.TextSize = 24
title.TextColor3 = Color3.fromRGB(232, 238, 248)
title.TextXAlignment = Enum.TextXAlignment.Left
title.Parent = card

local body = Instance.new("TextLabel")
body.Name = "Body"
body.Size = UDim2.new(1, -32, 0, 20)
body.Position = UDim2.new(0, 16, 0, 52)
body.BackgroundTransparency = 1
body.Text = "No Aether in this chain."
body.TextSize = 15
body.TextColor3 = Color3.fromRGB(150, 165, 190)
body.TextXAlignment = Enum.TextXAlignment.Left
body.Parent = card

--- ZIndex decides paint order over declaration order, which is section 6 of
--- conformance/LAYOUT.md and the one thing a geometric runner cannot verify.
local swatch = Instance.new("Frame")
swatch.Name = "Swatch"
swatch.Size = UDim2.new(0, 56, 0, 56)
swatch.Position = UDim2.new(1, -16, 1, -16)
swatch.AnchorPoint = Vector2.new(1, 1)
swatch.BackgroundColor3 = Color3.fromRGB(70, 120, 200)
swatch.ZIndex = 3
swatch.Parent = card

local swatchCorner = Instance.new("UICorner")
swatchCorner.CornerRadius = UDim.new(0, 12)
swatchCorner.Parent = swatch

--- ZERO BEFORE THE FIRST LAYOUT, and the engine answers zero too. Computed
--- properties carry no default in the reflection database, so reading one before
--- anything has laid the tree out used to return nil and then fail at `.X` with a
--- message about indexing nil rather than about layout.
print("built", card.Name)
print("AbsoluteSize before layout:", card.AbsoluteSize.X, "(the engine says 0 too)")

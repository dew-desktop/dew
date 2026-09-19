--!strict
--[[
	Experimental Features -- GuiObject.BlendingMode, an experimental Dew
	extension with no Roblox equivalent (`dew_host::datamodel::extensions`).

	    cargo run -- examples/host/experimental-features

	Unlocking it costs one line in `dew.toml`, pinning the revision this mod
	was written against:

	    experimentalDatamodel = ["GuiObject.BlendingMode@1"]

	Three panels, each two overlapping squares with a different
	`Enum.BlendMode` on the top one. Alpha, the default, paints over whatever
	is beneath it -- the top square hides the bottom one wherever they
	overlap. Additive sums the two colours where they overlap, which
	brightens the seam toward white. Multiply combines them toward black.
	The three overlap regions are visibly different colours, which is the
	whole point of the property.

	There is no framework here, matching `examples/host/basic-widget`'s
	convention: `Instance.new`, property assignment and `Parent` are the
	whole vocabulary.
]]

local widget = assert(dew.Widget, "dew.toml must grant `widget`")

local root = widget({
	width = 480,
	height = 220,
	anchor = "topRight",
	offset = { x = 24, y = 24 },
})

local backdrop = Instance.new("Frame")
backdrop.Name = "Backdrop"
backdrop.Size = UDim2.new(1, 0, 1, 0)
backdrop.BackgroundColor3 = Color3.fromRGB(18, 22, 30)
backdrop.Parent = root

--- One panel: a label naming the mode, a red square, and a green square on
--- top of it carrying that mode. `base` is parented before `top`, and
--- siblings with the same (default) `ZIndex` paint in that declaration
--- order, so `top` is what covers `base` in the overlap.
local function panel(index: number, name: string, mode: any)
	local x = 16 + (index - 1) * 152

	local label = Instance.new("TextLabel")
	label.Name = `{name}Label`
	label.Size = UDim2.new(0, 136, 0, 20)
	label.Position = UDim2.new(0, x, 0, 8)
	label.BackgroundTransparency = 1
	label.Text = name
	label.TextColor3 = Color3.fromRGB(233, 239, 249)
	label.TextSize = 14
	label.Parent = backdrop

	local base = Instance.new("Frame")
	base.Name = `{name}Base`
	base.Size = UDim2.new(0, 100, 0, 100)
	base.Position = UDim2.new(0, x, 0, 40)
	base.BackgroundColor3 = Color3.fromRGB(220, 40, 40)
	base.Parent = backdrop

	local top = Instance.new("Frame")
	top.Name = `{name}Top`
	top.Size = UDim2.new(0, 100, 0, 100)
	top.Position = UDim2.new(0, x + 36, 0, 76)
	top.BackgroundColor3 = Color3.fromRGB(40, 200, 90)
	top.BlendingMode = mode
	top.Parent = backdrop
end

panel(1, "Alpha", Enum.BlendMode.Alpha)
panel(2, "Additive", Enum.BlendMode.Additive)
panel(3, "Multiply", Enum.BlendMode.Multiply)

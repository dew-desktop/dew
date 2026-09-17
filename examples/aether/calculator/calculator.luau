--!strict
--[[
	Calculator — a Dew widget.

	The smaller of the two reference mods, and deliberately written in the SAME
	idiom as the larger one. It used to speak a different API entirely
	(`registerWidget` with a `{ text, subtext, icon, color }` descriptor the host
	rendered in a house style), which meant Dew had two rendering paths to keep
	correct and this widget's tree was not liftable anywhere.

	The shorthand is still worth having — most small widgets want it — but as a
	Luau helper that BUILDS one of these trees, not as a second path through the
	renderer. It is not written yet, because one consumer is not enough evidence
	of the shape it should take.

	DECLARES NO PERMISSIONS BEYOND A SURFACE. It does arithmetic on its own state
	and touches nothing else.

	THIS APPLET MOUNTS ITS OWN FRAMEWORK: it asks the host for a surface, builds
	its tree into it and hands the host nothing to mount on its behalf.
]]

local Aether = require("./roblox_packages/aether")
local vide = require("./roblox_packages/vide")

local create = Aether.create
local source = Aether.source
local derive = Aether.derive

--- ANCHORED LOWER, so both reference widgets can be on screen at once and
--- neither has to be moved to see the other.
local float = assert(dew.Float, "dew.toml must grant `float`")
local root = float({
	width = 260,
	height = 56,
	anchor = "topRight",
	offset = { x = 24, y = 104 },
})

local function build(): any
	local expression = source("")
	local result = source("")

	local display = derive(function()
		local shown = result()
		if shown ~= "" then
			return shown
		end
		local typed = expression()
		return if typed ~= "" then typed else "Calc"
	end)

	--- Evaluate whatever has been typed.
	---
	--- NO `loadstring`. The VM does not have one — the runtime removes it
	--- along with every other way to compile a chunk at runtime — so this
	--- parses the two-operand forms it supports and refuses the rest. A
	--- calculator that could evaluate arbitrary Luau would be an applet that runs
	--- arbitrary Luau on behalf of whoever typed into it.
	local function evaluate()
		local text = expression()
		local left, op, right = string.match(text, "^%s*(-?[%d%.]+)%s*([%+%-%*/])%s*(-?[%d%.]+)%s*$")
		local a, b = tonumber(left), tonumber(right)
		if not a or not b or not op then
			result("?")
			return
		end
		local value
		if op == "+" then
			value = a + b
		elseif op == "-" then
			value = a - b
		elseif op == "*" then
			value = a * b
		elseif b == 0 then
			result("÷0")
			return
		else
			value = a / b
		end
		result(string.format("%.10g", value))
	end

	local function typed(char: string)
		result("")
		expression(expression() .. char)
	end

	local function clear()
		expression("")
		result("")
	end

	--- Reachable once a keyboard or a row of buttons drives them; declared here
	--- so the tree below can grow into either without a second migration.
	local _actions = { typed = typed, evaluate = evaluate, clear = clear }

	return create "Frame" {
		Name = "Calculator",
		Size = UDim2.new(1, 0, 1, 0),
		BackgroundColor3 = Color3.fromHex("#121722"),

		create "UICorner" { CornerRadius = UDim.new(0, 14) },

		create "TextLabel" {
			Name = "Badge",
			Position = UDim2.new(0, 18, 0, 18),
			Size = UDim2.new(0, 44, 0, 20),
			BackgroundColor3 = Color3.fromHex("#f59e0b"),
			Text = "CALC",
			TextSize = 11,
			TextColor3 = Color3.fromHex("#0b0f16"),

			create "UICorner" { CornerRadius = UDim.new(0, 10) },
		},

		create "TextLabel" {
			--- 27, not 24: an 18pt line is 1.5 x TextSize tall.
			Name = "Display",
			Position = UDim2.new(0, 76, 0, 15),
			Size = UDim2.new(0, 166, 0, 27),
			BackgroundTransparency = 1,
			Text = display,
			TextSize = 18,
			TextColor3 = Color3.fromHex("#f8fafc"),
		},
	}
end

vide.mount(build, root)

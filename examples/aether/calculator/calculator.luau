--!strict
--[[
	Calculator -- a Dew widget built with Aether.

	    cargo run -- examples/aether/calculator

	Type digits and an operator, press equals. It parses the two-operand forms it
	supports and refuses the rest.

	It declares no capabilities beyond the surface it draws on, so `dew` arrives
	carrying only the ungated clock. That is deny-by-default working, and it is
	worth having one applet in the tree that shows it.
]]

local Aether = require("./roblox_packages/aether")

local create = Aether.create
local source = Aether.source
local derive = Aether.derive

local WIDTH, HEIGHT = 260, 56

--- Anchored lower than the other reference widget, so both can be on screen at
--- once and neither has to be moved to see the other.
local widget = assert(dew.widget, "dew.toml must grant `widget`")
local root = widget({
	width = WIDTH,
	height = HEIGHT,
	anchor = "topRight",
	offset = { x = 24, y = 104 },
})

local function build()
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

		local tree = create "Frame" {
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

		return tree
	end

local mounted = Aether.Desktop.Mount(build, WIDTH, HEIGHT)
;(mounted.Tree :: any).Parent = root

--!strict
--[[
	Dialog demo -- milestone 60 sprint B2's own live consumer.

	A real Dialog primitive (src/primitives/Dialog.luau) opened over existing
	background content: createDialog's focus trap, Presence's enter/exit
	transition, and ScreenStack's stacking, all composed rather than any one
	reimplemented here. A second dialog, opened from a button INSIDE the
	first, proves the stacking claim rather than merely asserting it -- the
	milestone's own trap is "does the OLD trap deactivate while a new one is
	active", and the only way to see that is two real dialogs open at once.

	UNCONTROLLED, THE SAME SHAPE Dialog.luau ITSELF NOW USES: `OutDialog`
	hands this file the real machine, and the button that opens each dialog
	calls `dialog.Open(trigger)` on it directly -- no boolean source pushed
	in from outside, see Dialog.luau's own header for why.

	WHAT TO LOOK FOR, screenshotted:
	  1. closed      -- only the background content and the "Open Dialog"
	                    button are visible.
	  2. dialog A    -- the backdrop dims the background, Dialog A's own
	                    surface draws over it, and its close button and the
	                    "Open nested dialog" button are both real, clickable
	                    controls.
	  3. stacked     -- Dialog B draws OVER Dialog A's own surface (both
	                    still open -- Modal is not Exclusive), proving a
	                    second dialog does not fight the first for the
	                    backdrop.
	  4. Escape      -- with both open, Escape closes ONLY Dialog B (the
	                    topmost); a second Escape then closes Dialog A --
	                    `isTopmostModal` in Dialog.luau is what makes this
	                    true rather than both traps hearing every keystroke.
]]

local Aether = require("./roblox_packages/aether")
local vide = require("./roblox_packages/vide")

local WIDTH, HEIGHT = 420, 300

local widget = assert(desktop.Widget, "dew.toml must grant `widget`")
local root = widget({ width = WIDTH, height = HEIGHT })

vide.root(function()
	local dialogA: any = nil
	local dialogB: any = nil
	local openButtonRef: any = nil

	Aether.create("Frame")({
		Parent = root,
		Name = "Root",
		Size = UDim2.fromScale(1, 1),
		BackgroundColor3 = Color3.fromRGB(24, 26, 34),
		BorderSizePixel = 0,

		Aether.create("TextLabel")({
			Name = "Background",
			Size = UDim2.new(1, -32, 0, 24),
			Position = UDim2.new(0, 16, 0, 16),
			BackgroundTransparency = 1,
			Text = "Background content -- still here once the dialog opens",
			TextColor3 = Color3.fromRGB(200, 205, 215),
			TextXAlignment = Enum.TextXAlignment.Left,
			TextSize = 14,
		}),

		Aether.Pressable({
			Name = "OpenDialogButton",
			Position = UDim2.new(0, 16, 0, 56),
			Size = UDim2.fromOffset(140, 32),
			BackgroundColor3 = Color3.fromRGB(60, 110, 200),
			OnActivated = function()
				dialogA.Open(openButtonRef)
			end,

			vide.action(function(ref: any)
				openButtonRef = ref
			end),

			Aether.create("UICorner")({ CornerRadius = UDim.new(0, 6) }),
			Aether.create("TextLabel")({
				Size = UDim2.fromScale(1, 1),
				BackgroundTransparency = 1,
				Text = "Open Dialog",
				TextColor3 = Color3.fromRGB(240, 240, 240),
				TextSize = 13,
				Font = Enum.Font.GothamBold,
			}),
		}),

		Aether.Dialog({
			Id = "dialog-a",
			Name = "DialogA",
			OutDialog = function(d: any)
				dialogA = d
			end,
			Title = "Dialog A",

			Children = function(dialog: any)
				return Aether.create("Frame")({
					Name = "Content",
					Size = UDim2.new(1, -32, 1, -80),
					Position = UDim2.new(0, 16, 0, 48),
					BackgroundTransparency = 1,

					Aether.create("TextLabel")({
						Size = UDim2.new(1, 0, 0, 20),
						BackgroundTransparency = 1,
						Text = "This is Dialog A's own content.",
						TextColor3 = Color3.fromRGB(220, 220, 220),
						TextXAlignment = Enum.TextXAlignment.Left,
						TextSize = 12,
					}),

					Aether.Pressable({
						Name = "OpenNestedButton",
						Position = UDim2.new(0, 0, 0, 32),
						Size = UDim2.fromOffset(160, 28),
						BackgroundColor3 = Color3.fromRGB(70, 90, 130),
						OnActivated = function()
							dialogB.Open()
						end,

						vide.action(function(ref: any)
							if ref then
								dialog.RegisterFocusable("open-nested", ref)
							end
						end),

						Aether.create("UICorner")({ CornerRadius = UDim.new(0, 4) }),
						Aether.create("TextLabel")({
							Size = UDim2.fromScale(1, 1),
							BackgroundTransparency = 1,
							Text = "Open nested dialog",
							TextColor3 = Color3.fromRGB(235, 235, 235),
							TextSize = 12,
						}),
					}),
				})
			end,
		}),

		Aether.Dialog({
			Id = "dialog-b",
			Name = "DialogB",
			OutDialog = function(d: any)
				dialogB = d
			end,
			Title = "Dialog B (nested)",
			Size = UDim2.fromOffset(260, 140),
			ZIndex = 200,

			Children = function()
				return Aether.create("TextLabel")({
					Size = UDim2.new(1, -32, 0, 20),
					Position = UDim2.new(0, 16, 0, 48),
					BackgroundTransparency = 1,
					Text = "Dialog B stacks over Dialog A.",
					TextColor3 = Color3.fromRGB(220, 220, 220),
					TextXAlignment = Enum.TextXAlignment.Left,
					TextSize = 12,
				})
			end,
		}),
	})
end)
--!strict
--[[
	The Dew dashboard -- Library, Store, and Account in one window
	(milestone 25), grown from milestone 23's marketplace-only browser. The
	Store tab below is that original browser, moved under a tab and
	otherwise untouched; Library and Account are built on `dew.Library` and
	`dew.Account` (ADR-017/ADR-018). Account's own sign-in form (milestone
	26 sprint 3) is what finally lets this window replace `manage.rs`'s
	Win32 one, rather than only reading state that window still had to
	create.

	Bundled with the host rather than installed like a marketplace package
	-- `dew.toml` asks for `discover`, `install`, `library` and `auth`,
	`Capability::Host` permissions no ordinary applet can ever be granted.
]]

local vide = require("./roblox_packages/vide")
local source = vide.source
local effect = vide.effect
local create = vide.create

local Aether = require("./roblox_packages/aether")
local Tabs = Aether.Tabs

local window = assert(desktop.Window, "dew.toml must grant `window`")
local marketplace = assert(dew.Marketplace, "dew.toml must grant `discover` and `install`")
local library = assert(dew.Library, "dew.toml must grant `library`")
local account = assert(dew.Account, "dew.toml must grant `auth`")

-- TALLER BY EXACTLY THE TAB BAR'S OWN HEIGHT (see `TAB_BAR_HEIGHT` below),
-- so each tab's content frame keeps the 420x520 the Store tab's layout was
-- always built against, rather than that layout having to shrink to make
-- room for tabs above it.
local root = window({ title = "Dew Dashboard", width = 420, height = 560 })

-- `dom.luau`'s own factory, parenting a new "Aether recipe" `StyleSheet`
-- under `root` immediately (see that file's header for why this can't wait
-- until a later sprint's first `recipe()` call). Constructed here, used by
-- no recipe yet -- sprints 3+ are what actually calls `recipe()` against it.
local newDom = require("./dom")
local dom = newDom(root)

-- THE DESIGN TOKEN SHEET (milestone 30 sprint 3): every color and radius
-- this file hard-codes today, extracted once into a plain `StyleSheet` of
-- named `Attribute`s -- the same shape `styled-pressable-recipe`'s own
-- `sunrise`/`dusk` sheets use (that example's own header: "still plain
-- StyleSheets carrying only Attributes -- that half of the hand-built
-- pattern was already correct"). `dom.New` is used rather than a bare
-- `Instance.new("StyleSheet")` because it already parents a StyleSheet
-- under `root` the instant it is created (see `dom.luau`'s own header for
-- why that parenting can't wait) -- this sheet gets the same treatment
-- rather than a second, hand-rolled parenting path.
--
-- EVERY VALUE BELOW IS A STRAIGHT COPY of this file's own existing
-- `Color3.fromRGB`/`UDim.new` literals, not a new palette -- picking new
-- values is explicitly left to the person (see milestone 30's own Open
-- Question 1), not decided by this sprint. NOTHING READS THESE YET: no
-- `recipe()` call exists anywhere in this file this sprint, so every
-- literal a token below replaces is still also written by hand at its own
-- call site -- migrating call sites onto `$TokenName` starts sprint 4 (the
-- tab bar).
local design = dom.New("StyleSheet")
design.Name = "Design"
-- Dashboard body background (`body.BackgroundColor3`, line ~89 below).
design:SetAttribute("BackgroundColor", Color3.fromRGB(18, 20, 26))
-- Row background (Library/Store rows) and input background (email/password
-- TextBoxes) -- one recurring surface tone, not three separate ones.
design:SetAttribute("SurfaceColor", Color3.fromRGB(28, 31, 40))
-- Library/Store row `UICorner.CornerRadius` -- the only radius this file
-- ever sets.
design:SetAttribute("RowRadius", UDim.new(0, 8))
-- Primary text: every panel's own title, each row's own name label, and
-- (the same value, same role) the email/password TextBoxes' own input text.
design:SetAttribute("TextPrimary", Color3.fromRGB(233, 239, 249))
-- Secondary text: status/description labels, the Store row's `Meta` line,
-- the sign-in form's own field labels.
design:SetAttribute("TextSecondary", Color3.fromRGB(146, 162, 188))
-- TextBox placeholder text (email/password).
design:SetAttribute("TextPlaceholder", Color3.fromRGB(110, 122, 140))
-- "Default" tone button background: Library/Store Refresh, Toggle, Sign In.
design:SetAttribute("AccentColor", Color3.fromRGB(38, 52, 74))
-- Text paired with AccentColor above, and also the active tab's own label
-- color (the tab label never actually changes color, only its background
-- does -- see the `effect` below).
design:SetAttribute("AccentText", Color3.fromRGB(206, 222, 246))
-- "Danger" tone button background: Uninstall, Sign Out.
design:SetAttribute("DangerColor", Color3.fromRGB(90, 48, 48))
-- Text paired with DangerColor above.
design:SetAttribute("DangerText", Color3.fromRGB(240, 214, 214))
-- "Success" tone button background: Install -- a third, genuinely distinct
-- tone from AccentColor (Toggle's own default look), not a duplicate of it.
design:SetAttribute("SuccessColor", Color3.fromRGB(48, 90, 64))
-- Text paired with SuccessColor above.
design:SetAttribute("SuccessText", Color3.fromRGB(214, 240, 224))
-- The sign-in error label's own text color -- a distinct literal from
-- DangerText above (a button's own text vs. an error message's own), kept
-- as its own token rather than merged.
design:SetAttribute("ErrorText", Color3.fromRGB(224, 148, 148))
-- Active/inactive tab background.
design:SetAttribute("TabActiveColor", Color3.fromRGB(48, 66, 94))
design:SetAttribute("TabInactiveColor", Color3.fromRGB(26, 29, 38))

type Package = { ownerUserId: string, appletId: string, uploadedAt: string }
type LibraryEntry = { id: string, name: string, description: string, enabled: boolean, running: boolean }

-- `colour: Color3?` (milestone 30 sprint 7) -- `nil` leaves `TextColor3`
-- unset rather than writing a literal. THIS MATTERS, NOT COSMETIC:
-- `Dom::styled_property` (`cascade.rs`) reads an instance's own EXPLICIT
-- property first and only falls back to a matching `StyleRule` when nothing
-- was set directly -- an explicit `TextColor3` here would always beat a
-- recipe's own `Bind` for the same property, the same reason the tab bar's
-- own `BackgroundColor3` was removed from `tabButton` when sprint 4 gave it
-- a recipe. Every EXISTING call site still passes a real `Color3` and is
-- unaffected; only the Account panel's own labels (sprint 7) pass `nil` and
-- bind a `textRecipe` afterward instead.
local function label(
	parent: Instance,
	name: string,
	position: UDim2,
	size: UDim2,
	textSize: number,
	colour: Color3?
): TextLabel
	local text = Instance.new("TextLabel")
	text.Name = name
	text.Position = position
	text.Size = size
	text.BackgroundTransparency = 1
	text.TextSize = textSize
	if colour then
		text.TextColor3 = colour
	end
	text.TextXAlignment = Enum.TextXAlignment.Left
	text.Parent = parent
	return text
end

local function button(parent: Instance, name: string, position: UDim2, size: UDim2, text: string, bg: Color3, fg: Color3): TextButton
	local b = Instance.new("TextButton")
	b.Name = name
	b.Position = position
	b.Size = size
	b.BackgroundColor3 = bg
	b.TextColor3 = fg
	b.TextSize = 12
	b.Text = text
	b.Parent = parent
	return b
end

local body = Instance.new("Frame")
body.Name = "Body"
body.Size = UDim2.new(1, 0, 1, 0)
body.BackgroundColor3 = Color3.fromRGB(18, 20, 26)
body.Parent = root

--------------------------------------------------------------------------
-- THE TAB BAR, on `Aether.Tabs` as of milestone 54 sprint 1 (`Aether.Core.
-- createTabs` directly, before that) -- the hand-rolled `activeTab` source
-- and its effect are gone; `tabs.Active` is the same idea, backed by a
-- machine that also enforces exactly one active tab and answers a
-- Space/Return press or an arrow key while a tab button holds keyboard
-- selection, neither of which the hand-rolled version ever had. The effect
-- below is still the only thing that reacts to it, the same split the Store
-- tab's own `packages` source already uses.
--------------------------------------------------------------------------

local TAB_BAR_HEIGHT = 40

type TabName = "library" | "store" | "account"

-- THE TAB RECIPE (milestone 30 sprint 4) -- this dashboard's own first real
-- `recipe()` call. One variant dimension, `state`, resolving to the same two
-- tokens sprint 3 already put on `design` (`$TabActiveColor`/
-- `$TabInactiveColor`, themselves exact copies of the old
-- `ACTIVE_TAB_COLOR`/`INACTIVE_TAB_COLOR` literals this recipe replaces).
-- `SetThemeSource(design)` is called ONCE, EAGERLY, RIGHT HERE -- before the
-- `effect` below ever calls `Bind` -- because the shared `StyleDerive` that
-- call creates is what `$TabActiveColor`/`$TabInactiveColor` resolve
-- against; a `Bind` reaching the cascade before this line would resolve
-- both tokens to nothing (`createRecipe.luau`'s own header, THEMING
-- section).
local tabRecipe = Aether.Core.createRecipe({
	base = { BackgroundColor3 = "$TabInactiveColor" },
	variants = {
		state = {
			active = { BackgroundColor3 = "$TabActiveColor" },
			inactive = {},
		},
	},
}, { Dom = dom, Component = "DashboardTab" })
tabRecipe.SetThemeSource(design)

local tabBar = Instance.new("Frame")
tabBar.Name = "TabBar"
tabBar.Position = UDim2.new(0, 0, 0, 0)
tabBar.Size = UDim2.new(1, 0, 0, TAB_BAR_HEIGHT)
tabBar.BackgroundTransparency = 1
tabBar.Parent = body

-- The tabs sit in one row, 8 in from the bar's top left corner and 4 apart.
-- Each button's `LayoutOrder` keeps them in declaration order, since the list
-- would otherwise sort them by Name.
local tabBarPadding = Instance.new("UIPadding")
tabBarPadding.PaddingLeft = UDim.new(0, 8)
tabBarPadding.PaddingTop = UDim.new(0, 8)
tabBarPadding.Parent = tabBar

local tabBarLayout = Instance.new("UIListLayout")
tabBarLayout.FillDirection = Enum.FillDirection.Horizontal
tabBarLayout.SortOrder = Enum.SortOrder.LayoutOrder
tabBarLayout.Padding = UDim.new(0, 4)
tabBarLayout.Parent = tabBar

-- `Aether.Tabs` HARD-WIRES `PointerRouter`/`SelectionRouter` (milestone 54
-- sprint 1), so this dashboard no longer needs to name either. `Source` is
-- still handed over explicitly, though, and that is still load bearing:
-- left at Aether's own default, `tabs.Active` would change correctly but
-- the `effect` below would never re-run, since a vide source only notifies
-- the reactive graph that created it, and this dashboard vendors its own
-- vide rather than sharing Aether's. `Action` for the same reason -- the
-- bound props below are real `vide.action` values, and only THIS dashboard's
-- own `create` (via its own vide) can apply one to an element it makes.
local tabs = Tabs({
	Source = source,
	Action = vide.action,
	Tabs = { "library", "store", "account" } :: { TabName },
	InitialActive = "library" :: TabName,
})

-- BUILT WITH `create` (this dashboard's own, from its own vide package),
-- NOT `button()`, because `GetRootProps`'s bound actions are real
-- `vide.action` values now, not plain functions -- the manual
-- `ipairs`/`entry(b)` loop this replaced could not have called them
-- (`type(entry) == "function"` is false for one). `create` already knows how
-- to apply a `vide.action`, a plain property and a child in one pass, which
-- is what makes the loop unnecessary rather than merely hidden elsewhere.
local function tabButton(name: TabName, text: string, order: number): TextButton
	return create("TextButton")(tabs.GetTab(name).GetRootProps({
		Name = `Tab{text}`,
		LayoutOrder = order,
		Size = UDim2.new(0, 128, 0, 28),
		-- BackgroundColor3 IS NOT SET HERE ANY MORE -- `tabRecipe.Bind`,
		-- called on every one of these three buttons inside the `effect`
		-- below (which runs once, immediately, before this script's first
		-- real paint), is what resolves it through the cascade now.
		BackgroundTransparency = 0,
		TextColor3 = Color3.fromRGB(206, 222, 246),
		TextSize = 12,
		Text = text,
		Parent = tabBar,
	})) :: TextButton
end

local libraryTabButton = tabButton("library", "Library", 1)
local storeTabButton = tabButton("store", "Store", 2)
local accountTabButton = tabButton("account", "Account", 3)

-- CONTENT FRAMES, ALL THE SAME RECT, ONE VISIBLE AT A TIME -- toggling
-- `Visible` rather than destroying and rebuilding a tab's tree every
-- switch, which would re-run `Discover()` on every visit to Store.
local function contentFrame(name: string): Frame
	local f = Instance.new("Frame")
	f.Name = name
	f.Position = UDim2.new(0, 0, 0, TAB_BAR_HEIGHT)
	f.Size = UDim2.new(1, 0, 1, -TAB_BAR_HEIGHT)
	f.BackgroundTransparency = 1
	f.Parent = body
	return f
end

local libraryTab = contentFrame("LibraryTab")
local storeTab = contentFrame("StoreTab")
local accountTab = contentFrame("AccountTab")

vide.root(function()
	effect(function()
		local tab = tabs.Active()
		libraryTab.Visible = tab == "library"
		storeTab.Visible = tab == "store"
		accountTab.Visible = tab == "account"

		local highlight = function(b: TextButton, active: boolean)
			tabRecipe.Bind(b, { state = if active then "active" else "inactive" })
		end
		highlight(libraryTabButton, tab == "library")
		highlight(storeTabButton, tab == "store")
		highlight(accountTabButton, tab == "account")
	end)
end)

--------------------------------------------------------------------------
-- LIBRARY: every installed applet, with a way to launch, uninstall, and
-- enable/disable it. `dew.Library`'s four calls are all synchronous (see
-- `capabilities.rs`), so there is nothing here to poll -- a call's result
-- is read the instant it returns, the same as any ordinary function.
--------------------------------------------------------------------------

label(libraryTab, "Title", UDim2.new(0, 16, 0, 12), UDim2.new(1, -112, 0, 24), 18, Color3.fromRGB(233, 239, 249)).Text =
	"Library"

local libraryRefreshButton = button(
	libraryTab,
	"Refresh",
	UDim2.new(1, -96, 0, 12),
	UDim2.new(0, 80, 0, 24),
	"Refresh",
	Color3.fromRGB(38, 52, 74),
	Color3.fromRGB(206, 222, 246)
)

local libraryStatus =
	label(libraryTab, "Status", UDim2.new(0, 16, 0, 44), UDim2.new(1, -32, 0, 40), 12, Color3.fromRGB(146, 162, 188))
libraryStatus.TextWrapped = true
libraryStatus.TextYAlignment = Enum.TextYAlignment.Top

local libraryList = Instance.new("ScrollingFrame")
libraryList.Name = "List"
libraryList.Position = UDim2.new(0, 16, 0, 92)
libraryList.Size = UDim2.new(1, -32, 1, -108)
libraryList.BackgroundTransparency = 1
libraryList.CanvasSize = UDim2.new(1, 0, 0, 0)
libraryList.AutomaticCanvasSize = Enum.AutomaticSize.Y
libraryList.Parent = libraryTab

local libraryListLayout = Instance.new("UIListLayout")
libraryListLayout.Padding = UDim.new(0, 8)
libraryListLayout.Parent = libraryList

local LIBRARY_ROW_HEIGHT = 84

-- THE LIBRARY ROW RECIPE (milestone 30 sprint 5) -- `base`-only, no variants,
-- since every row looks the same: the same one-rule-two-instances pattern
-- `styled-pressable-recipe.luau`'s own `corner`/`button` binding already
-- proved (one resolved `StyleRule` carrying both `BackgroundColor3` and
-- `CornerRadius`, bound to a Frame and a UICorner with the SAME empty
-- selection, so both share the one generated tag -- safe because
-- `resolve_full` is not filtered against a target's own declared properties,
-- `cascade.rs` line 38). Resolves sprint 3's own `$SurfaceColor`/`$RowRadius`
-- tokens.
--
-- NO `SetThemeSource` CALL HERE, OR BELOW. `tabRecipe.SetThemeSource(design)`
-- above already created and pointed the one shared `StyleDerive` this `dom`
-- will ever have -- `createRecipe.luau`'s own `stateFor` is keyed by `Dom`
-- (line 189), and `setThemeSource`'s body (lines 202-212) always resolves
-- `stateFor(dom)` before touching `state.Derive`. Every recipe below shares
-- this same `dom` table, so it shares that same `DomState`, including the
-- derive tabRecipe already pointed at `design`. A second `SetThemeSource
-- (design)` call from either recipe below would only re-point the SAME
-- derive at the SAME sheet -- a harmless no-op, but not a call this file
-- needs to make twice.
local libraryRowRecipe = Aether.Core.createRecipe({
	base = { BackgroundColor3 = "$SurfaceColor", CornerRadius = "$RowRadius" },
}, { Dom = dom, Component = "LibraryRow" })

-- THE DASHBOARD BUTTON RECIPE (milestone 30 sprint 5, renamed from
-- `libraryButtonRecipe` in sprint 6 once Store's own Install button became
-- its second consumer -- `LibraryButton` undersold what this now covers).
-- One variant dimension, `tone` (`default`/`danger`/`success`), each
-- resolving BOTH `BackgroundColor3` (consumed by the `Aether.Pressable`
-- Frame) AND `TextColor3` (consumed by its own child `TextLabel`, since
-- `Pressable` returns a plain Frame with no `.Text` of its own -- confirmed
-- by reading `Pressable.luau` in full, and by `styled-pressable-recipe.luau`'s
-- own `SwapTheme` button, which parents a child `Aether.create("TextLabel")`
-- rather than setting a `.Text` property that does not exist). Same
-- one-rule-two-consumers trick as the row recipe above: the Frame reads
-- `BackgroundColor3` off the resolved rule, the label reads `TextColor3` off
-- the SAME rule, bound with the SAME selection.
--
-- `success` (Store's own Install button, sprint 6) is a genuinely third tone
-- neither `default` (Library's Toggle) nor `danger` (Library's Uninstall,
-- Account's Sign Out) covers -- resolving sprint 3's own `$SuccessColor`/
-- `$SuccessText` tokens, not a duplicate of either existing tone.
local buttonRecipe = Aether.Core.createRecipe({
	variants = {
		tone = {
			default = { BackgroundColor3 = "$AccentColor", TextColor3 = "$AccentText" },
			danger = { BackgroundColor3 = "$DangerColor", TextColor3 = "$DangerText" },
			success = { BackgroundColor3 = "$SuccessColor", TextColor3 = "$SuccessText" },
		},
	},
}, { Dom = dom, Component = "DashboardButton" })

-- ONE VIDE ROOT PER ROW (milestone 30 sprint 5), TRACKED HERE SO A REBUILD
-- CAN TEAR IT DOWN FIRST. `Aether.Pressable` calls `vide.cleanup
-- (machine.Destroy)` internally (`Pressable.luau` line 97), which errors
-- ("cannot cleanup outside a stable or reactive scope") with no active vide
-- scope -- confirmed by reading vide's own `cleanup.luau` directly -- so
-- `buildLibraryRow` below wraps its own Pressable-creating work in
-- `vide.root`, the same shape `styled-pressable-recipe.luau`'s own header
-- names as load-bearing ("everything below runs inside one").
--
-- THAT SCOPE'S OWN DESTRUCTOR IS NOT CALLED AUTOMATICALLY when a row's plain
-- `Instance:Destroy()` runs below. Confirmed by reading `PointerRouter.luau`
-- in full: it has no `Destroying`/`AncestryChanged` listener of its own, and
-- vide's own cleanup registration is one-directional -- a scope tears down
-- its OWN registered callbacks when ITS OWN destructor runs; it does not
-- listen for a Roblox instance going away. Without this table mapping each
-- row Frame to its own scope destructor, every library refresh (every
-- Toggle/Uninstall click, every external `dew install`/`dew uninstall`, any
-- `library.OnChange` firing for any reason) would leak one stale
-- `PointerRouter` registration per button per row, forever.
local libraryRowRoots: { [Frame]: () -> () } = {}

local function clearLibraryRows()
	for _, child in ipairs(libraryList:GetChildren()) do
		if child:IsA("Frame") and child.Name == "Row" then
			local destroyRowScope = libraryRowRoots[child]
			if destroyRowScope then
				-- TEAR DOWN THE SCOPE FIRST: this runs both buttons' own
				-- `machine.Destroy` (each one unregisters that button from
				-- `PointerRouter`) BEFORE the row instance itself is gone.
				destroyRowScope()
				libraryRowRoots[child] = nil
			end
			child:Destroy()
		end
	end
end

-- FORWARD-DECLARED: `buildLibraryRow`'s own buttons call `rebuildLibraryRows`
-- to rebuild the list after each action, and `rebuildLibraryRows` calls
-- `buildLibraryRow` for each entry -- the two close over each other.
--
-- SEPARATE FROM `refreshLibrary` BELOW ON PURPOSE. A row's own action
-- already wrote its own result into `libraryStatus.Text` (`Enabled 'x'`,
-- an error, ...); rebuilding the row list must not then overwrite that
-- with the empty-store notice, which is `refreshLibrary`'s job for the
-- Refresh button and the initial load, neither of which has a result of
-- its own to preserve.
local rebuildLibraryRows: () -> number

--[[
	DESIGN NOTE, REVISIT IF THIS STOPS FEELING RIGHT: only Enable/Disable
	and Uninstall are shown here -- there used to be a third "Launch"
	button, removed after it turned out to just be confusing rather than
	load-bearing.

	`enabled` (persisted in `installed.json`, "load this automatically at
	the next service start") and "is this actually running right now" are
	genuinely two different axes -- `dew.Library.SetEnabled` did not always
	touch the second one. It used to be a disk write ONLY: an applet could
	be enabled and running, enabled and NOT running (closed by hand, or
	before this session's own live-toggle fix, simply "not yet restarted"),
	or disabled while still running until the next restart. "Launch" was
	the one way to force it running right now without touching the
	persisted bit, which is why it existed as its own button next to
	Enable/Disable rather than folded into it.

	Once `SetEnabled` started live-loading/unloading (so the two axes stay
	in sync the instant you click), the only case Launch still covered on
	its own was an applet still `enabled` but no longer running -- closed
	by hand, or crashed -- where the toggle would read "Disable" (since
	`enabled` says so) and clicking it would turn the thing OFF, not bring
	it back. That is confusing on its face: two controls that mostly do the
	same thing, and the one time they diverge, the more prominent one
	(Enable/Disable) does the wrong thing.

	THE FIX: `LibraryEntry.running` (from `dew.Library.List`, backed by
	`coordinator::running_snapshot()`, not `installed.json`) is what this
	toggle's label and action are keyed on now, not `entry.enabled`. Off
	(not running) always reads "Enable" and always starts it; on (running)
	always reads "Disable" and always stops it. `SetEnabled(id, true)` on
	an enabled-but-stopped applet is exactly the old `Launch`'s job --
	start it without changing a bit that was already correct -- so nothing
	is lost, and the enabled-but-stopped case now shows the right label
	instead of the wrong one.
]]
local function buildLibraryRow(entry: LibraryEntry)
	-- WRAPPED IN `vide.root` (see `libraryRowRoots`'s own comment above for
	-- why this is load-bearing, not decorative): everything inside,
	-- including both `Aether.Pressable` calls, runs inside a real reactive
	-- scope, and that scope's own destructor is handed back out as
	-- `destroyRowScope` for `clearLibraryRows` to call before this row's
	-- `Instance:Destroy()`.
	local destroyRowScope, row = vide.root(function(): Frame
		local row = Instance.new("Frame")
		row.Name = "Row"
		row.Size = UDim2.new(1, 0, 0, LIBRARY_ROW_HEIGHT)
		row.Parent = libraryList

		local corner = Instance.new("UICorner")
		corner.Parent = row
		-- SAME SELECTION ({}), SAME TAG, SAME RULE -- see `libraryRowRecipe`'s
		-- own comment above.
		libraryRowRecipe.Bind(row, {})
		libraryRowRecipe.Bind(corner, {})

		label(row, "Name", UDim2.new(0, 12, 0, 8), UDim2.new(1, -24, 0, 18), 14, Color3.fromRGB(233, 239, 249)).Text =
			entry.name
		local description = label(
			row,
			"Description",
			UDim2.new(0, 12, 0, 28),
			UDim2.new(1, -24, 0, 16),
			11,
			Color3.fromRGB(146, 162, 188)
		)
		description.Text = if entry.description == "" then entry.id else entry.description

		-- NEITHER HANDLER REBUILDS THE ROW LIST ITSELF (milestone 26 sprint 2;
		-- both used to call `rebuildLibraryRows()` here directly). `library.OnChange`
		-- below does that now, for every change regardless of source -- these
		-- two clicks are just one more source of one, the same as a terminal
		-- running `dew install` is. Only the STATUS MESSAGE is this handler's
		-- own to set, since `OnChange` has no way to know what a caller's own
		-- action was trying to do.
		--
		-- CLICK HANDLING IS `OnActivated` NOW, NOT `.Activated:Connect` --
		-- `Aether.Pressable` is a Frame, not a `TextButton`; it has no
		-- `.Activated` signal of its own. Behavior is otherwise verbatim.
		local toggleButton = Aether.Pressable({
			Parent = row,
			Name = "Toggle",
			Position = UDim2.new(0, 12, 0, 52),
			Size = UDim2.new(0, 100, 0, 24),
			BackgroundTransparency = 0,
			OnActivated = function()
				local result = library.SetEnabled(entry.id, not entry.running)
				libraryStatus.Text = if result.ok
					then `{if entry.running then "Disabled" else "Enabled"} '{entry.id}'`
					else result.error
			end,
		})
		local toggleLabel = Instance.new("TextLabel")
		toggleLabel.Name = "Label"
		toggleLabel.Size = UDim2.fromScale(1, 1)
		toggleLabel.BackgroundTransparency = 1
		toggleLabel.TextSize = 12
		toggleLabel.Text = if entry.running then "Disable" else "Enable"
		toggleLabel.Parent = toggleButton
		buttonRecipe.Bind(toggleButton, { tone = "default" })
		buttonRecipe.Bind(toggleLabel, { tone = "default" })

		-- THE ONE DELIBERATE BEHAVIOR CHANGE THIS MILESTONE MAKES (sprint 8):
		-- Uninstall no longer removes anything itself -- it opens a real
		-- confirmation `Aether.Dialog`, and the ACTUAL `library.Uninstall`
		-- call moved into that dialog's own Confirm button below. This
		-- `Dialog` is built here, inside `buildLibraryRow`'s own `vide.root`
		-- (see that scope's own comment above) rather than needing any new
		-- cleanup machinery: `libraryRowRoots`'s existing destructor already
		-- tears down every machine THIS scope registers on rebuild, and
		-- `Dialog.luau`'s own `cleanup(function() dialog.Destroy() ... end)`
		-- (confirmed by reading the file directly) is exactly one more such
		-- registration, torn down the same way, at the same time, as the two
		-- `Aether.Pressable`s already are.
		local uninstallDialog: any = nil
		-- FORWARD-DECLARED, THE SAME REASON `uninstallDialog` ABOVE IS:
		-- Confirm's own `OnActivated` (inside `Aether.Dialog`'s `Children`
		-- closure, below) needs to reach this Frame to destroy it, but that
		-- closure is DEFINED while `Aether.Dialog({...})` is still being
		-- constructed -- before a `local uninstallDialogFrame = Aether.Dialog
		-- ({...})` statement's own right-hand side has finished, the name it
		-- introduces is not yet in scope for anything nested inside that
		-- same expression (the same one-statement forward-reference gotcha
		-- `rebuildLibraryRows`/`refreshAccount` are already forward-declared
		-- above to avoid). Declaring it `nil` here first, then assigning
		-- (not `local`-declaring) it below, makes it a real upvalue instead.
		local uninstallDialogFrame: any = nil

		local uninstallButton = Aether.Pressable({
			Parent = row,
			Name = "Uninstall",
			Position = UDim2.new(0, 124, 0, 52),
			Size = UDim2.new(0, 88, 0, 24),
			BackgroundTransparency = 0,
			-- OPENS THE CONFIRMATION DIALOG NOW, INSTEAD OF UNINSTALLING
			-- DIRECTLY. `dialog.Open(trigger)` is handed this very button --
			-- the same `dialogA.Open(openButtonRef)` shape
			-- `examples/aether/dialog-demo/dialog-demo.luau` already
			-- demonstrates -- so `createDialog`'s own focus trap has
			-- something real to restore focus to on close.
			OnActivated = function()
				uninstallDialog.Open(uninstallButton)
			end,
		})
		local uninstallLabel = Instance.new("TextLabel")
		uninstallLabel.Name = "Label"
		uninstallLabel.Size = UDim2.fromScale(1, 1)
		uninstallLabel.BackgroundTransparency = 1
		uninstallLabel.TextSize = 12
		uninstallLabel.Text = "Uninstall"
		uninstallLabel.Parent = uninstallButton
		buttonRecipe.Bind(uninstallButton, { tone = "danger" })
		buttonRecipe.Bind(uninstallLabel, { tone = "danger" })

		-- THE CONFIRMATION DIALOG ITSELF. `Dialog`'s own default surface
		-- chrome (`createDialog.luau`'s header, the Frame `Dialog.luau`
		-- draws at line ~291) already carries a real `BackgroundColor3`,
		-- corner, stroke, and close button -- confirmed by reading
		-- `Dialog.luau` in full before writing this -- so nothing here
		-- restyles the surface itself; only the Cancel/Confirm row below,
		-- which this dashboard DOES own, goes through the existing
		-- `buttonRecipe` tones the same way every other button in this file
		-- already does.
		--
		-- `.Parent` IS SET EXPLICITLY, RIGHT AFTER, BECAUSE `Aether.Dialog`
		-- NEVER DOES THIS ITSELF -- confirmed live, the hard way: an
		-- earlier version of this sprint called `Aether.Dialog({...})` as a
		-- bare statement here and the dialog opened (`ScreenStack`/
		-- `createDialog`'s own logical state flipped correctly) with
		-- NOTHING VISIBLE ON SCREEN. Reading `Dialog.luau`'s own body again
		-- explains why: its root `create("Frame")(dialog.GetRootProps({
		-- Name = ..., Size = ..., Position = ..., BackgroundTransparency =
		-- 1, Visible = ..., ZIndex = ... }))` call builds that fixed
		-- overrides table itself and never reads `props.Parent` (or any
		-- other arbitrary key) back out of the `DialogProps` it was handed
		-- -- the `[any]: any` in that type only satisfies the type checker,
		-- it is not a real passthrough. `dialog-demo.luau`'s own usage
		-- never hits this: it nests `Aether.Dialog({...})` calls directly
		-- inside another `Aether.create("Frame")({ ..., Aether.Dialog({...})
		-- })`'s own children array, which is how VIDE's `create` parents a
		-- returned element in that example, a shape this row's plain
		-- `Instance.new`-built tree does not use. Parented to `body` (the
		-- whole window's own content frame, 420x560), not `row` (84px
		-- tall): the dialog's own `Size = UDim2.fromScale(1, 1)` backdrop
		-- must cover the entire window, the same area `dialog-demo.luau`'s
		-- own root Frame covers, not just this one row's small rectangle.
		uninstallDialogFrame = Aether.Dialog({
			Id = `uninstall-{entry.id}`,
			Name = "UninstallConfirm",
			Title = "Uninstall applet?",
			Size = UDim2.fromOffset(300, 160),
			OutDialog = function(d: any)
				uninstallDialog = d
			end,

			Children = function(dialog: any)
				local content = Instance.new("Frame")
				content.Name = "Content"
				content.Size = UDim2.new(1, -32, 1, -80)
				content.Position = UDim2.new(0, 16, 0, 48)
				content.BackgroundTransparency = 1

				-- A LITERAL COLOR, NOT `textRecipe.Bind` -- `textRecipe` (sprint
				-- 7) is declared further down this file, in the Account
				-- section, and is therefore not a legal upvalue here: Luau
				-- resolves a local at the textual point of its own
				-- declaration, and referencing it from this earlier function
				-- would silently capture a global (nil) instead, the same
				-- footgun this file's own `rebuildLibraryRows`/`refreshAccount`
				-- comments already name for the identical reason. Matches the
				-- row's own `Name`/`Description` labels immediately above,
				-- which are the same kind of still-literal leftover sprint 5's
				-- own report already flagged, not a fresh oversight introduced
				-- here.
				local message = label(content, "Message", UDim2.new(0, 0, 0, 0), UDim2.new(1, 0, 0, 40), 12, Color3.fromRGB(233, 239, 249))
				message.Text = `This removes '{entry.name}' from this machine.`
				message.TextWrapped = true

				local cancelButton = Aether.Pressable({
					Parent = content,
					Name = "Cancel",
					Position = UDim2.new(0, 0, 1, -28),
					Size = UDim2.new(0, 88, 0, 24),
					BackgroundTransparency = 0,
					OnActivated = function()
						dialog.Close()
					end,
				})
				local cancelLabel = Instance.new("TextLabel")
				cancelLabel.Name = "Label"
				cancelLabel.Size = UDim2.fromScale(1, 1)
				cancelLabel.BackgroundTransparency = 1
				cancelLabel.TextSize = 12
				cancelLabel.Text = "Cancel"
				cancelLabel.Parent = cancelButton
				buttonRecipe.Bind(cancelButton, { tone = "default" })
				buttonRecipe.Bind(cancelLabel, { tone = "default" })

				local confirmButton = Aether.Pressable({
					Parent = content,
					Name = "Confirm",
					Position = UDim2.new(0, 96, 1, -28),
					Size = UDim2.new(0, 88, 0, 24),
					BackgroundTransparency = 0,
					-- THE ACTUAL `library.Uninstall` CALL, MOVED HERE
					-- VERBATIM from the old immediate `OnActivated` above --
					-- same result handling, same `libraryStatus.Text`
					-- wording.
					--
					-- `uninstallDialogFrame:Destroy()` IS EXPLICIT, NOT LEFT
					-- TO `dialog.Close()`'S OWN EXIT FADE, FOUND LIVE TO BE
					-- NECESSARY: `library.Uninstall` fires `library.OnChange`
					-- SYNCHRONOUSLY (`onLibraryChanged`/`rebuildLibraryRows`
					-- above), which calls this row's own `destroyRowScope()`
					-- -- tearing down the `vide.root` this Dialog was built
					-- inside -- as part of the SAME click handler that is
					-- still running. `dialog.Close()` alone only flips
					-- `ScreenStack`'s logical state and starts `Presence`'s
					-- reactive fade-out effect; that effect needs a later
					-- tick to actually flip the Frame's own `Visible` to
					-- false, and this row's own scope is destroyed before
					-- that tick ever runs. The Frame itself is undamaged by
					-- that teardown -- it is parented to `body`, not to
					-- `row`, exactly so the dialog can cover the whole
					-- window (see this Dialog's own comment above) -- so
					-- nothing else ever destroys it either, and it was found
					-- live stuck on screen, fully opaque, forever. Destroying
					-- it here, directly, the instant Confirm is pressed,
					-- is honest about what is actually happening: this row
					-- and its dialog are BOTH going away regardless, so an
					-- immediate removal rather than a cosmetic fade that can
					-- never complete is the correct behavior, not a
					-- workaround. `dialog.Close()` still runs first, so
					-- `ScreenStack`/`createDialog`'s own bookkeeping closes
					-- cleanly while the scope is still alive, before the
					-- destroy that follows.
					OnActivated = function()
						dialog.Close()
						local result = library.Uninstall(entry.id)
						libraryStatus.Text = if result.ok then `Uninstalled '{entry.id}'` else result.error
						uninstallDialogFrame:Destroy()
					end,
				})
				local confirmLabel = Instance.new("TextLabel")
				confirmLabel.Name = "Label"
				confirmLabel.Size = UDim2.fromScale(1, 1)
				confirmLabel.BackgroundTransparency = 1
				confirmLabel.TextSize = 12
				confirmLabel.Text = "Uninstall"
				confirmLabel.Parent = confirmButton
				buttonRecipe.Bind(confirmButton, { tone = "danger" })
				buttonRecipe.Bind(confirmLabel, { tone = "danger" })

				return content
			end,
		})
		uninstallDialogFrame.Parent = body

		return row
	end)
	libraryRowRoots[row] = destroyRowScope
end

rebuildLibraryRows = function(): number
	local entries = library.List() :: { LibraryEntry }
	clearLibraryRows()
	for _, entry in ipairs(entries) do
		buildLibraryRow(entry)
	end
	return #entries
end

local function refreshLibrary()
	local count = rebuildLibraryRows()
	libraryStatus.Text = if count == 0 then "No applets installed yet -- see the Store tab" else ""
end

-- THE ONE PLACE THE ROW LIST ACTUALLY GETS REBUILT, other than an explicit
-- Refresh click or the initial load below. Fires for a change from ANY
-- source: this dashboard's own Launch/Toggle/Uninstall, a crash, a plain
-- tray unload, or a separate `dew install`/`dew uninstall` run from a
-- terminal against this same running service (milestone 26) -- the point
-- of building it this way rather than re-polling on a timer.
--
-- NEVER TOUCHES `libraryStatus.Text` EXCEPT WHEN THE LIST IS EMPTY.
-- Clobbering it on every change would erase the result message a button's
-- own handler just set, one frame later -- the exact bug fixed earlier this
-- milestone for the manual-refresh case, reachable again here if this
-- called `refreshLibrary()` instead of `rebuildLibraryRows()` directly.
local function onLibraryChanged()
	local count = rebuildLibraryRows()
	if count == 0 then
		libraryStatus.Text = "No applets installed yet -- see the Store tab"
	end
end

library.OnChange(onLibraryChanged)

libraryRefreshButton.Activated:Connect(refreshLibrary)

--------------------------------------------------------------------------
-- STORE: milestone 23's marketplace browser, unchanged beyond parenting
-- into `storeTab` instead of `body` directly.
--------------------------------------------------------------------------

label(storeTab, "Title", UDim2.new(0, 16, 0, 12), UDim2.new(1, -112, 0, 24), 18, Color3.fromRGB(233, 239, 249)).Text =
	"Marketplace"

local refreshButton = button(
	storeTab,
	"Refresh",
	UDim2.new(1, -96, 0, 12),
	UDim2.new(0, 80, 0, 24),
	"Refresh",
	Color3.fromRGB(38, 52, 74),
	Color3.fromRGB(206, 222, 246)
)

-- WRAPPED, AND GIVEN ROOM FOR MORE THAN ONE LINE: a network or auth error
-- (see `platform::describe_auth_error`) is a full sentence, not a label,
-- and an unwrapped one just runs off the edge of the window rather than
-- being clipped -- which reads as "blank" at a glance, not as an error.
local status =
	label(storeTab, "Status", UDim2.new(0, 16, 0, 44), UDim2.new(1, -32, 0, 40), 12, Color3.fromRGB(146, 162, 188))
status.TextWrapped = true
status.TextYAlignment = Enum.TextYAlignment.Top

local list = Instance.new("ScrollingFrame")
list.Name = "List"
list.Position = UDim2.new(0, 16, 0, 92)
list.Size = UDim2.new(1, -32, 1, -108)
list.BackgroundTransparency = 1
-- A ROW IS SIZED AGAINST THE CANVAS, NOT THE FRAME'S OWN VISIBLE BOX --
-- `render.rs` lays every child out inside `CanvasSize`'s box, and that
-- defaults to zero on both axes. Without this, `UDim2.new(1, 0, 0, ...)`
-- on a row resolves its width against a zero-wide canvas: every row
-- exists, at zero width, which is indistinguishable from nothing at all.
-- Width tracks the frame itself; height grows to fit whatever the list
-- layout below actually lays out.
list.CanvasSize = UDim2.new(1, 0, 0, 0)
list.AutomaticCanvasSize = Enum.AutomaticSize.Y
list.Parent = storeTab

local listLayout = Instance.new("UIListLayout")
listLayout.Padding = UDim.new(0, 8)
listLayout.Parent = list

local ROW_HEIGHT = 64

-- STATE FIRST, EVERYTHING ELSE AFTER. `buildRow` below closes over
-- `onInstalled`; declaring it after `buildRow` would make that a capture of
-- a global instead of this local, since a Luau chunk resolves an upvalue at
-- the point a function is DEFINED, not where it is called.
local packages = source(nil :: { Package }?)

-- CALLBACKS, NOT AWAITING FLAGS (milestone 26; was a flag polled from
-- `desktop.Clock.OnFrame` through milestone 25 -- see `capabilities.rs` for
-- why that bookkeeping moved into the host). `Discover`/`Install` call these
-- back exactly once, themselves, the instant an answer lands; nothing here
-- polls for it any more.
local function onDiscovered(discovered)
	if discovered.ok then
		packages(discovered.packages)
		status.Text = if #discovered.packages == 0 then "No public packages yet" else ""
	else
		status.Text = discovered.error
	end
end

local function onInstalled(installed)
	status.Text = if installed.ok then `Installed '{installed.id}'` else installed.error
	-- THE LIBRARY TAB IS WHAT SHOWS AN INSTALL'S RESULT, since a newly
	-- installed applet is a Library entry, not a Store one -- refreshed
	-- here too so switching tabs right after an install shows it without a
	-- manual Refresh click.
	if installed.ok then
		refreshLibrary()
	end
end

local function clearRows()
	for _, child in ipairs(list:GetChildren()) do
		if child:IsA("Frame") and child.Name == "Row" then
			child:Destroy()
		end
	end
end

-- STORE'S ROW REUSES `libraryRowRecipe` DIRECTLY (milestone 30 sprint 6,
-- resolving Open Question 2): the row Frame + UICorner here is structurally
-- identical to Library's own (same `$SurfaceColor`/`$RowRadius` tokens, same
-- one-rule-two-instances shape), and sprint 5's own report already named
-- this as shareable-as-built rather than needing a second, identical
-- recipe. `Meta`'s own `published {date}` line has no Library equivalent,
-- but that difference lives in a plain `TextLabel` below, never in the row
-- recipe itself, so it does not make sharing awkward.
--
-- NO PER-ROW `vide.root` HERE, UNLIKE `buildLibraryRow`. That machinery
-- exists there because `buildLibraryRow` is called with no wrapping
-- reactive scope at all -- `Aether.Pressable`'s own `vide.cleanup` call
-- would error otherwise (`graph.luau`'s own `assert_stable_scope`/`cleanup`
-- helper, confirmed by reading both directly). `buildRow` has no such gap:
-- it is only ever called from inside the `effect` below, which is itself a
-- live vide scope for the whole time its callback runs (`push_scope(node)`
-- brackets the call in `graph.luau`'s own `evaluate_node`) -- so the new
-- `Aether.Pressable`'s `vide.cleanup(machine.Destroy)` registers against
-- THAT effect node, not nothing. And on the effect's own next run (any
-- `packages()` change), `evaluate_node` calls `flush_cleanups(node)` before
-- invoking the callback again, which runs every `machine.Destroy` this
-- row's own Pressable registered on the PREVIOUS run, before `clearRows`
-- even destroys the row Instances. Adding `libraryRowRoots`-style tracking
-- here would duplicate a cleanup path vide's own effect re-run already
-- provides for free -- confirmed live by reading `graph.luau` in full, not
-- assumed from Library's own shape.
local function buildRow(pkg: Package)
	local row = Instance.new("Frame")
	row.Name = "Row"
	row.Size = UDim2.new(1, 0, 0, ROW_HEIGHT)
	row.Parent = list

	local corner = Instance.new("UICorner")
	corner.Parent = row
	-- SAME SELECTION ({}), SAME TAG, SAME RULE -- see `libraryRowRecipe`'s
	-- own comment above.
	libraryRowRecipe.Bind(row, {})
	libraryRowRecipe.Bind(corner, {})

	label(row, "Name", UDim2.new(0, 12, 0, 8), UDim2.new(1, -100, 0, 18), 14, Color3.fromRGB(233, 239, 249)).Text =
		`@{pkg.ownerUserId}/{pkg.appletId}`
	label(row, "Meta", UDim2.new(0, 12, 0, 30), UDim2.new(1, -100, 0, 16), 11, Color3.fromRGB(146, 162, 188)).Text =
		`published {pkg.uploadedAt}`

	-- CLICK HANDLING IS `OnActivated` NOW, NOT `.Activated:Connect` --
	-- `Aether.Pressable` is a Frame, not a `TextButton`; it has no
	-- `.Activated` signal of its own. Behavior is otherwise verbatim.
	local install = Aether.Pressable({
		Parent = row,
		Name = "Install",
		Position = UDim2.new(1, -84, 0.5, -12),
		Size = UDim2.new(0, 72, 0, 24),
		BackgroundTransparency = 0,
		OnActivated = function()
			status.Text = `Installing {pkg.appletId}...`
			marketplace.Install(pkg.ownerUserId, pkg.appletId, onInstalled)
		end,
	})
	local installLabel = Instance.new("TextLabel")
	installLabel.Name = "Label"
	installLabel.Size = UDim2.fromScale(1, 1)
	installLabel.BackgroundTransparency = 1
	installLabel.TextSize = 12
	installLabel.Text = "Install"
	installLabel.Parent = install
	buttonRecipe.Bind(install, { tone = "success" })
	buttonRecipe.Bind(installLabel, { tone = "success" })
end

-- THE SOURCE IS THE ONLY STATE; THE EFFECT IS THE ONLY PLACE THAT REACTS,
-- the same split `widget-behaviors` already uses. `desktop.Clock.OnFrame`
-- below only senses and pushes; rebuilding the row list is this effect's
-- job alone.
vide.root(function()
	effect(function()
		local current = packages()
		clearRows()
		if current == nil then
			return
		end
		for _, pkg in ipairs(current) do
			buildRow(pkg)
		end
	end)
end)

refreshButton.Activated:Connect(function()
	status.Text = "Refreshing..."
	marketplace.Discover(onDiscovered)
end)

status.Text = "Loading..."
marketplace.Discover(onDiscovered)

--------------------------------------------------------------------------
-- ACCOUNT: who is signed in, a way to sign out, and -- new this milestone
-- -- a way to sign IN, without a terminal. `dew.Account.SignIn` is
-- trigger-with-callback (the same shape Sprint 1 gave `Discover`/`Install`),
-- since `platform::login` is a real network call.
--
-- KNOWN LIMITATION, WORTH REVISITING: the password field below is a plain
-- `TextBox`, not masked. This DataModel's own `TextBox` has no password/
-- masking property the way `manage.rs`'s native `ES_PASSWORD` Win32 field
-- did -- carrying that behavior over means intercepting every keystroke and
-- substituting a placeholder character while tracking the real value
-- separately, which is real work this sprint did not do. Flagged rather
-- than silently shipped.
--------------------------------------------------------------------------

-- THE TEXT RECIPE (milestone 30 sprint 7) -- every plain `TextLabel` in this
-- panel that still carried a literal `TextColor3` (Title, the signed-in
-- Status line, the sign-in form's own field labels, the error label).
-- `base`-less, four `tone`s, one property each -- there is no shared
-- property across tones the way `buttonRecipe`'s own `BackgroundColor3`/
-- `TextColor3` pair is shared, so a `base` table would have nothing to
-- hold. Library/Store's own row `Name`/`Description` labels are NOT
-- touched here -- they are still-literal leftovers sprint 5/6 left in
-- place (confirmed by reading both commits' own messages: neither names
-- the row labels at all, so this reads as an oversight of scope, not a
-- deliberate decision, and is flagged here for a later cleanup rather than
-- fixed in this sprint, which is Account's own).
local textRecipe = Aether.Core.createRecipe({
	variants = {
		tone = {
			primary = { TextColor3 = "$TextPrimary" },
			secondary = { TextColor3 = "$TextSecondary" },
			accent = { TextColor3 = "$AccentText" },
			error = { TextColor3 = "$ErrorText" },
		},
	},
}, { Dom = dom, Component = "DashboardText" })

-- THE TEXT INPUT RECIPE (milestone 30 sprint 7) -- a plain `TextBox` tagged
-- and bound exactly like Library's own row `Frame`/`UICorner` (sprint 5),
-- NOT an `Aether.TextInput` swap (the implementation plan's own sprint 7
-- row names this explicitly: that file offers no capability this dashboard
-- needs beyond what a bare `TextBox` already has). `base`-only, no variants
-- -- both fields look identical.
--
-- `PlaceholderColor3` IS INCLUDED, INVESTIGATED FIRST, NOT ASSUMED SAFE:
-- `StyleRule:SetProperty` (`members.rs`, confirmed by reading it directly)
-- takes an arbitrary property NAME with no allowlist -- `coerce_variant_value`
-- only converts the Lua VALUE, and `cascade.rs`'s own `resolve_full` doc
-- says outright its map "is not filtered against the target's own declared
-- properties." Nothing in the cascade stops this token from resolving.
-- SEPARATELY, THOUGH: `render.rs`'s own paint path never reads
-- `PlaceholderColor3` or `PlaceholderText` at all (confirmed by reading
-- `node()`/`text_for()`/`colour()` in full -- `text_for` reads only the
-- `Text` property, filtered non-empty; there is no placeholder-text
-- rendering in this host today, a real, separate gap from the password-
-- masking one this file already names below). Binding the token here is
-- therefore correct and harmless -- it resolves through the cascade
-- exactly like `BackgroundColor3`/`TextColor3` do -- but produces no
-- visible difference from the literal it replaces, because neither one
-- ever painted anything to begin with. Kept as a real `$TextPlaceholder`
-- binding anyway, for the same reason `BackgroundColor3`/`TextColor3`
-- are: consistency with the token sheet, not because this specific
-- property is currently visible.
local textInputRecipe = Aether.Core.createRecipe({
	base = {
		BackgroundColor3 = "$SurfaceColor",
		TextColor3 = "$TextPrimary",
		PlaceholderColor3 = "$TextPlaceholder",
	},
}, { Dom = dom, Component = "DashboardTextInput" })

local title = label(accountTab, "Title", UDim2.new(0, 16, 0, 12), UDim2.new(1, -32, 0, 24), 18, nil)
title.Text = "Account"
textRecipe.Bind(title, { tone = "primary" })

-- SIGNED-IN STATE.
local accountStatus = label(accountTab, "Status", UDim2.new(0, 16, 0, 52), UDim2.new(1, -32, 0, 24), 13, nil)
accountStatus.TextWrapped = true
textRecipe.Bind(accountStatus, { tone = "accent" })

-- FORWARD-DECLARED (same reason `rebuildLibraryRows` is, above): both
-- buttons below are built inside a `vide.root` whose own `OnActivated`
-- closures close over `refreshAccount`/`signInInFlight`/`signInLabel` --
-- declaring any of those AFTER the closures that reference them would
-- capture a global instead of this local, since a Luau chunk resolves an
-- upvalue at the point a function is DEFINED, not where it runs.
local signOutButton: Frame
local signInButton: Frame
local signInLabel: TextLabel
local refreshAccount: () -> ()

-- GUARDED THE SAME WAY `manage.rs`'s OWN `SIGNIN_IN_FLIGHT` GUARDED ITS
-- button: a second click before the first request returns is ignored
-- rather than starting a second sign-in, and the button's own label says
-- so while one is outstanding.
local signInInFlight = false

-- SIGNED-OUT STATE: a plain sign-in form. Built once, alongside the
-- signed-in controls above, and toggled by `refreshAccount` -- the same
-- "both sets exist, only one is visible" shape `manage.rs`'s own
-- `create_login_section` already used for the identical two states.
local emailLabel = label(accountTab, "EmailLabel", UDim2.new(0, 16, 0, 52), UDim2.new(0, 70, 0, 20), 12, nil)
emailLabel.Text = "Email:"
textRecipe.Bind(emailLabel, { tone = "secondary" })

local emailBox = Instance.new("TextBox")
emailBox.Name = "Email"
emailBox.Position = UDim2.new(0, 90, 0, 50)
emailBox.Size = UDim2.new(1, -106, 0, 24)
emailBox.PlaceholderText = "you@example.com"
emailBox.TextSize = 12
emailBox.ClearTextOnFocus = false
emailBox.Text = ""
emailBox.Parent = accountTab
textInputRecipe.Bind(emailBox, {})

local passwordLabel = label(accountTab, "PasswordLabel", UDim2.new(0, 16, 0, 84), UDim2.new(0, 70, 0, 20), 12, nil)
passwordLabel.Text = "Password:"
textRecipe.Bind(passwordLabel, { tone = "secondary" })

local passwordBox = Instance.new("TextBox")
passwordBox.Name = "Password"
passwordBox.Position = UDim2.new(0, 90, 0, 82)
passwordBox.Size = UDim2.new(1, -106, 0, 24)
passwordBox.PlaceholderText = "Password"
passwordBox.TextSize = 12
passwordBox.ClearTextOnFocus = false
passwordBox.Text = ""
passwordBox.Parent = accountTab
textInputRecipe.Bind(passwordBox, {})

local signInError = label(accountTab, "SignInError", UDim2.new(0, 16, 0, 152), UDim2.new(1, -32, 0, 40), 11, nil)
signInError.Text = ""
signInError.TextWrapped = true
signInError.TextYAlignment = Enum.TextYAlignment.Top
textRecipe.Bind(signInError, { tone = "error" })

local function setSignInFormVisible(visible: boolean)
	emailLabel.Visible = visible
	emailBox.Visible = visible
	passwordLabel.Visible = visible
	passwordBox.Visible = visible
	signInButton.Visible = visible
	signInError.Visible = visible
end

-- BOTH BUTTONS NEED AN ACTIVE VIDE SCOPE TO CONSTRUCT AT ALL (milestone 30
-- sprint 7, the same requirement `libraryRowRoots`'s own comment names in
-- detail for sprint 5): `Aether.Pressable` calls `vide.cleanup
-- (machine.Destroy)` internally, which errors ("cannot cleanup outside a
-- stable or reactive scope") with no active scope.
--
-- UNLIKE LIBRARY'S ROWS, NEITHER BUTTON IS EVER DESTROYED OR REBUILT.
-- Confirmed by reading every place this file touches `signOutButton`/
-- `signInButton`: `refreshAccount` below only ever flips `.Visible`, and
-- nothing anywhere calls `:Destroy()` on either one. A single `vide.root`
-- here, whose own destructor is simply never called, is enough -- the
-- dashboard window itself owns this whole panel's lifetime, and there is
-- no rebuild path (no `library.OnChange`-shaped signal for account state)
-- that would need `libraryRowRoots`-style per-instance tracking to avoid
-- leaking a stale `PointerRouter` registration.
vide.root(function()
	-- CLICK HANDLING IS `OnActivated` NOW, NOT `.Activated:Connect` --
	-- `Aether.Pressable` is a Frame, not a `TextButton`; it has no
	-- `.Activated` signal of its own. Behavior is otherwise verbatim.
	signOutButton = Aether.Pressable({
		Parent = accountTab,
		Name = "SignOut",
		Position = UDim2.new(0, 16, 0, 84),
		Size = UDim2.new(0, 96, 0, 28),
		BackgroundTransparency = 0,
		OnActivated = function()
			account.SignOut()
			refreshAccount()
		end,
	})
	local signOutLabel = Instance.new("TextLabel")
	signOutLabel.Name = "Label"
	signOutLabel.Size = UDim2.fromScale(1, 1)
	signOutLabel.BackgroundTransparency = 1
	signOutLabel.TextSize = 12
	signOutLabel.Text = "Sign out"
	signOutLabel.Parent = signOutButton
	buttonRecipe.Bind(signOutButton, { tone = "danger" })
	buttonRecipe.Bind(signOutLabel, { tone = "danger" })

	signInButton = Aether.Pressable({
		Parent = accountTab,
		Name = "SignIn",
		Position = UDim2.new(0, 16, 0, 116),
		Size = UDim2.new(0, 96, 0, 28),
		BackgroundTransparency = 0,
		OnActivated = function()
			if signInInFlight then
				return
			end
			local email = emailBox.Text
			local password = passwordBox.Text
			if email == "" or password == "" then
				signInError.Text = "Enter an email and password"
				return
			end

			signInInFlight = true
			signInLabel.Text = "Signing in..."
			signInError.Text = ""

			account.SignIn(email, password, function(result)
				signInInFlight = false
				signInLabel.Text = "Sign in"
				if result.ok then
					passwordBox.Text = ""
					refreshAccount()
				else
					signInError.Text = result.error
				end
			end)
		end,
	})
	signInLabel = Instance.new("TextLabel")
	signInLabel.Name = "Label"
	signInLabel.Size = UDim2.fromScale(1, 1)
	signInLabel.BackgroundTransparency = 1
	signInLabel.TextSize = 12
	signInLabel.Text = "Sign in"
	signInLabel.Parent = signInButton
	buttonRecipe.Bind(signInButton, { tone = "default" })
	buttonRecipe.Bind(signInLabel, { tone = "default" })
end)

refreshAccount = function()
	local session = account.Whoami()
	if session then
		accountStatus.Text = `Signed in as {session.email}`
		accountStatus.Visible = true
		signOutButton.Visible = true
		setSignInFormVisible(false)
	else
		accountStatus.Visible = false
		signOutButton.Visible = false
		setSignInFormVisible(true)
	end
end

refreshAccount()

--------------------------------------------------------------------------
-- INITIAL STATE, for the tab that opens active by default.
--------------------------------------------------------------------------

refreshLibrary()

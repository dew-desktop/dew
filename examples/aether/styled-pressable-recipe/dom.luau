--!strict
--[[
	The real `Dom` port `createRecipe.luau`'s own header names as a host's to
	write, not that file's: `Instance.new`, `.Selector =`, `:SetProperty`,
	`.Parent =`, `CollectionService:AddTag`/`RemoveTag`, and (A4)
	`.StyleSheet =` on a `StyleDerive` for `SetThemeSource`.

	Every one of these matches `../styled-pressable/styled-pressable.luau`'s
	own hand-built calls line for line (its lines 74-91, 106, 120) -- this
	module does not invent a different call shape for any of them, it only
	moves them behind the `Dom` port `createRecipe` is driven through.

	ONE THING `createRecipe.luau` DOES NOT DO ITSELF, confirmed by reading
	it in full: it never parents its own shared "Aether recipe" `StyleSheet`
	anywhere (it parents RULES and the DERIVE onto that sheet, never the
	sheet onto anything) -- and exposes no way for a caller to reach that
	sheet and parent it either (`Recipe`'s own type is `Resolve`/`Bind`/
	`Unbind`/`SetThemeSource`, nothing else). Confirmed live: without this,
	`cascade.rs`'s own `applicable_style_sheets` (which walks UP from the
	bound instance through its own ancestors' children looking for a
	`StyleSheet`) never finds it, and every `$Token` in a recipe's rules
	resolves to nothing -- the button rendered unthemed, corner square,
	before this adapter parented the sheet. THIS ADAPTER decides where:
	`New("StyleSheet")` parents a newly created sheet under `root` right
	away, the same place `styled-pressable.luau`'s own hand-built `design`
	sheet is parented (`design.Parent = root`) -- a host-adapter decision,
	not a change to `createRecipe.luau`'s own shape. See this sprint's own
	report for why this is flagged as a real, worth-reporting gap rather
	than quietly routed around.
]]

local CollectionService = (services :: any):GetService("CollectionService")

return function(root: any)
	local function New(className: string): any
		local instance = Instance.new(className)
		if className == "StyleSheet" then
			instance.Parent = root
		end
		return instance
	end

	local function SetParent(instance: any, parent: any)
		instance.Parent = parent
	end

	local function SetSelector(rule: any, selector: string)
		rule.Selector = selector
	end

	local function SetProperty(rule: any, name: string, value: any)
		rule:SetProperty(name, value)
	end

	local function AddTag(instance: any, tag: string)
		CollectionService:AddTag(instance, tag)
	end

	local function RemoveTag(instance: any, tag: string)
		CollectionService:RemoveTag(instance, tag)
	end

	--- A4's own port addition. An ORDINARY property write, not `SetProperty`
	--- (which sets a cascaded `StyleRule` property and bypasses reflection
	--- refusal to allow `$Token` strings through) -- `StyleDerive` has no
	--- `StyleRule` properties of its own to cascade. Matches
	--- `styled-pressable.luau`'s own `derive.StyleSheet = ...` (line 120)
	--- exactly.
	local function SetDeriveSource(derive: any, source: any)
		derive.StyleSheet = source
	end

	return {
		New = New,
		SetParent = SetParent,
		SetSelector = SetSelector,
		SetProperty = SetProperty,
		AddTag = AddTag,
		RemoveTag = RemoveTag,
		SetDeriveSource = SetDeriveSource,
	}
end

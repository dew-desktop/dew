--!strict
--[[
	The real `Dom` port `createRecipe.luau`'s own header names as a host's to
	write, not that file's: `Instance.new`, `.Selector =`, `:SetProperty`,
	`.Parent =`, `CollectionService:AddTag`/`RemoveTag`, and (A4)
	`.StyleSheet =` on a `StyleDerive` for `SetThemeSource`.

	Duplicated from `examples/aether/styled-pressable-recipe/dom.luau`, not
	promoted to a shared module -- no existing convention shares a Luau file
	between an example package and a top-level applet's own package (each is
	a fully self-contained pesde package with its own vendored
	`roblox_packages/`), and inventing one to save about 80 lines duplicated
	exactly once would be infrastructure nobody has asked for twice. Worth
	revisiting if a third real Dom adapter consumer shows up.

	ONE THING `createRecipe.luau` DOES NOT DO ITSELF, confirmed by reading
	it in full: it never parents its own shared "Aether recipe" `StyleSheet`
	anywhere (it parents RULES and the DERIVE onto that sheet, never the
	sheet onto anything) -- and exposes no way for a caller to reach that
	sheet and parent it either (`Recipe`'s own type is `Resolve`/`Bind`/
	`Unbind`/`SetThemeSource`, nothing else). Confirmed live (in the
	reference adapter this was duplicated from): without this,
	`cascade.rs`'s own `applicable_style_sheets` (which walks UP from the
	bound instance through its own ancestors' children looking for a
	`StyleSheet`) never finds it, and every `$Token` in a recipe's rules
	resolves to nothing. THIS ADAPTER decides where: `New("StyleSheet")`
	parents a newly created sheet under `root` right away, the same place
	the reference adapter's own hand-built `design` sheet is parented -- a
	host-adapter decision, not a change to `createRecipe.luau`'s own shape.
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

	--- An ORDINARY property write, not `SetProperty` (which sets a cascaded
	--- `StyleRule` property and bypasses reflection refusal to allow
	--- `$Token` strings through) -- `StyleDerive` has no `StyleRule`
	--- properties of its own to cascade.
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

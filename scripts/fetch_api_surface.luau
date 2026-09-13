--!strict
--[[
	Regenerate `host/datamodel/api_surface.json` -- the METHOD AND EVENT half of
	the DataModel Standard's surface.

	    lune run scripts/fetch_api_surface.luau

	WHY THIS EXISTS AT ALL
	`rbx_reflection_database`, which `datamodel-surface` already reads for the
	property surface, carries NO methods and NO events. Its `ClassDescriptor` has
	exactly `name`, `tags`, `superclass`, `properties` and `default_properties`;
	it is built for file serialisation, and a method cannot be serialised. So the
	obvious plan -- "same database, same tool" -- is not available, and the
	scriptable API surface had no machine-readable source in this project.

	Roblox's own API dump has it. This fetches that dump, keeps the fraction the
	standard is about, and writes it next to the tool that reads it.

	WHY A VENDORED SUBSET RATHER THAN THE DUMP
	The dump is 7.2 MB of 909 classes, and this standard is about roughly forty.
	Vendoring all of it would put six megabytes of chat windows and physics
	constraints in a UI repository forever. Vendoring NOTHING and fetching at
	build time is worse: the tool would report a different answer depending on
	when it ran, and an offline build would report none at all.

	So: a pinned subset, generated, with the build recorded in it. That is the
	same bargain `pesde.lock` makes.

	WHY IT IS PINNED RATHER THAN LIVE
	The build this writes is the build the standard is measured against, and it
	must be a fact rather than a moving target. `docs/datamodel_scope.md` and
	`conformance/cases/*.luau` already disagreed about which Roblox they describe
	-- 0.728.0.7280895 against 0.736.0.7361346, eight builds apart inside one
	standard -- because the property half tracked whatever the Rust crate shipped
	and nobody was told when it moved. Recording the version here is what lets
	`datamodel-surface` say so out loud.
]]

local fs = require("@lune/fs")
local net = require("@lune/net")
local process = require("@lune/process")
local serde = require("@lune/serde")

--- Roblox publishes no stable URL for a versioned dump, so this reads the
--- Client Tracker, which mirrors each build's dump beside the version string
--- that produced it. Both come from the same ref in one commit, which is what
--- makes the pair coherent rather than two fetches that might straddle a
--- release.
--- Pinned to commit 2f099c9d2db0061c81728bad66f91140f2b20fed for Roblox build
--- 0.736.0.7361346, which is what all verified conformance cases cite.
local TRACKER = "https://raw.githubusercontent.com/MaximumADHD/Roblox-Client-Tracker/2f099c9d2db0061c81728bad66f91140f2b20fed"
local OUT = "host/datamodel/api_surface.json"

--- The classes the standard covers. `GuiObject` descendants are found by walking
--- the hierarchy; these are the modifiers, which descend from `UIComponent` and
--- so are not reachable that way.
---
--- KEPT IN STEP WITH `MODIFIERS` in host/src/bin/datamodel_surface.rs BY HAND.
--- The two lists are in different languages and there is no shared home for
--- them; the tool fails loudly if this file lacks a class it wanted, which is
--- the cheapest available check that they have not drifted.
local MODIFIERS = {
	"UICorner",
	"UIPadding",
	"UIListLayout",
	"UIGridLayout",
	"UIPageLayout",
	"UITableLayout",
	"UIGradient",
	"UIStroke",
	"UIScale",
	"UIAspectRatioConstraint",
	"UISizeConstraint",
	"UITextSizeConstraint",
	"UIFlexItem",
}

print("fetching the API dump and its version")
local dumpRes = net.request(`{TRACKER}/API-Dump.json`)
if not dumpRes.ok then
	print(`could not fetch the dump: {dumpRes.statusCode} {dumpRes.statusMessage}`)
	process.exit(1)
end
local versionRes = net.request(`{TRACKER}/version.txt`)
if not versionRes.ok then
	print(`could not fetch the version: {versionRes.statusCode} {versionRes.statusMessage}`)
	process.exit(1)
end

--- The dump is served with a UTF-8 BOM, which `serde.decode` rejects as a value
--- the same way `serde_json` does. Stripped rather than tolerated downstream, so
--- the file this writes is clean for the Rust side.
local raw = string.gsub(dumpRes.body, "^\u{feff}", "")
local dump = serde.decode("json", raw)
local version = string.gsub(versionRes.body, "%s+$", "")
print(`Roblox {version}, {#dump.Classes} classes in the dump`)

local byName: { [string]: any } = {}
for _, class in ipairs(dump.Classes) do
	byName[class.Name] = class
end

--- Everything that IS or DESCENDS FROM GuiObject, plus the modifiers, plus every
--- ancestor of both.
---
--- ANCESTORS ARE NOT OPTIONAL. Members are declared on the class that introduces
--- them and are not repeated on descendants: `TextButton` itself declares one
--- function, and the forty-four a script can call on it come from `GuiButton`,
--- `GuiObject`, `GuiBase2d`, `GuiBase`, `Instance` and `Object`. A subset that
--- dropped those would report a surface of nearly nothing and look like good
--- news.
local wanted: { [string]: boolean } = {}
local function keepWithAncestors(name: string)
	local cursor: string? = name
	while cursor and byName[cursor] do
		if wanted[cursor] then
			break
		end
		wanted[cursor] = true
		cursor = byName[cursor].Superclass
	end
end

for _, class in ipairs(dump.Classes) do
	local cursor: string? = class.Name
	while cursor and byName[cursor] do
		if cursor == "GuiObject" then
			keepWithAncestors(class.Name)
			break
		end
		cursor = byName[cursor].Superclass
	end
end
for _, name in ipairs(MODIFIERS) do
	if not byName[name] then
		print(`the dump has no class named {name}; MODIFIERS is stale`)
		process.exit(1)
	end
	keepWithAncestors(name)
end

--- A member is IN if an applet author could call it.
---
--- `Security` gates that, and it is two shapes in one file: a string for
--- functions and events, a `{ Read, Write }` table for properties. Only `None`
--- is reachable from ordinary Luau, so `RobloxScriptSecurity` and
--- `PluginSecurity` members are dropped -- a standard that demanded
--- `SetTextFromInput` of an implementation would be describing Roblox's own
--- internals rather than describing a UI.
---
--- Deprecated members go for the reason the property side already gives: nobody
--- should implement against them.
local function reachable(member: any): boolean
	local security = member.Security
	local level = if type(security) == "table" then security.Read else security
	if level ~= "None" then
		return false
	end
	for _, tag in ipairs(member.Tags or {}) do
		if tag == "Deprecated" then
			return false
		end
	end
	return true
end

--- A property is IN if it is writable by guest scripts, non-deprecated, and
--- represents a storable value, enum, or instance reference.
local function writableProperty(member: any): (boolean, string?)
	if member.MemberType ~= "Property" then
		return false, nil
	end
	for _, tag in ipairs(member.Tags or {}) do
		if tag == "Deprecated" or tag == "ReadOnly" then
			return false, nil
		end
	end
	local security = member.Security
	local writeSec = if type(security) == "table" then security.Write else security
	if writeSec ~= "None" then
		return false, nil
	end
	local cat = member.ValueType and member.ValueType.Category
	if cat ~= "DataType" and cat ~= "Enum" and cat ~= "Primitive" and cat ~= "Class" then
		return false, nil
	end
	return true, member.ValueType.Name
end

local classes: { [string]: any } = {}
local names: { string } = {}
for name in pairs(wanted) do
	table.insert(names, name)
end
table.sort(names)

local functions, events, propCount = 0, 0, 0
for _, name in ipairs(names) do
	local class = byName[name]
	--- A MAP FROM NAME TO KIND, not a list of records. Lune cannot tell an empty
	--- array from an empty map -- both encode as `{}` -- and most classes here
	--- introduce no members of their own, so a list shape made the file
	--- undeserialisable on the Rust side for exactly the classes that carry
	--- nothing. A map is unambiguous when empty, and a member name is unique
	--- within a class anyway.
	local members: { [string]: string } = {}
	local properties: { [string]: string } = {}
	for _, member in ipairs(class.Members) do
		local kind = member.MemberType
		if (kind == "Function" or kind == "Event") and reachable(member) then
			members[member.Name] = kind
			if kind == "Function" then
				functions += 1
			else
				events += 1
			end
		end
		local ok, typeName = writableProperty(member)
		if ok and typeName then
			properties[member.Name] = typeName
			propCount += 1
		end
	end
	classes[name] = {
		superclass = class.Superclass,
		properties = properties,
		members = members,
	}
end

fs.writeDir("host/datamodel")
fs.writeFile(
	OUT,
	serde.encode("json", {
		version = version,
		--- Recorded so the file explains itself where it is read, rather than
		--- only in the script that wrote it.
		note = "Classes, properties, methods and events pinned from Roblox's API dump at "
			.. version
			.. ". Generated by scripts/fetch_api_surface.luau; do not edit by hand.",
		classes = classes,
	}, true)
)

print(
	`wrote {OUT}: {#names} classes, {propCount} properties declared, {functions} methods, {events} events, Roblox {version}`
)

--!strict
--[[
	What a Dew applet is handed, as the language server sees it.

	A DEFINITIONS FILE, not a module. It is loaded with `--definitions=` rather
	than required, which is what lets it declare a GLOBAL: `dew` is one, the way
	`game` is one in the engine this host is shaped after, and without this file
	an editor reports "Unknown global 'dew'" on every applet.

	Kept beside the host that produces it: `host/src/capabilities.rs` builds the
	table this describes and `host/src/applets.rs` installs it. If those two and
	this file disagree, they are wrong and this is stale.

	A SURFACE RETURNS `any` AND SHOULD RETURN `Instance`. Definition files are
	parsed one at a time and share no types with each other, so naming `Instance`
	here makes the whole file fail to load wherever the DataModel types are not in
	this same file, and an applet gets no `dew` at all rather than a loosely typed
	root. `declare class` would let this file define `Instance` itself and
	luau-lsp 1.69 does not accept it. This tightens when Dew ships type
	definitions for its own DataModel, which is the thing that should own them.
]]

type Anchor = "topLeft" | "topRight" | "bottomLeft" | "bottomRight" | "center"

type WidgetOptions = {
	width: number?,
	height: number?,
	--- Defaults to "topRight". An anchor rather than coordinates, because a
	--- desktop is not one size.
	anchor: Anchor?,
	--- Pixels from the anchored corner. Defaults to 24, 24.
	offset: { x: number, y: number }?,
	--- Let clicks reach whatever is behind, everywhere. An applet with a control
	--- wants this false, which is the default.
	clickThrough: boolean?,
}

type WindowOptions = {
	width: number?,
	height: number?,
	--- Defaults to the applet's `name` in dew.toml, falling back to its id.
	title: string?,
}

--- A widget the size of the desktop.
---
--- Clicks fall through wherever nothing was painted, so the root frame must set
--- `BackgroundTransparency = 1` or the overlay becomes a screen-sized sheet of
--- glass that swallows every click on the machine.
type OverlayOptions = {
	--- Above every other window. Defaults to true.
	topmost: boolean?,
	clickThrough: boolean?,
}

type Storage = {
	get: (key: string) -> string?,
	set: (key: string, value: string) -> (),
}

type Clipboard = {
	read: () -> string,
	write: (text: string) -> (),
}

type Notifications = {
	show: (message: string) -> (),
}

type Audio = {
	chime: (name: string) -> (),
}

type Clock = {
	--- Seconds since the epoch, as a float.
	now: () -> number,
}

--- The capability table an applet is given.
---
--- EVERY FIELD BUT `time` IS OPTIONAL, and present only if `dew.toml` declared
--- the matching permission. That is the point: what an applet cannot name it
--- cannot reach, and one declaring nothing gets a table carrying only the clock.
---
--- A SURFACE IS A FUNCTION HERE, not a block an applet returns. Calling it is
--- how the applet asks, and what comes back is the root to parent into. An
--- ungranted surface is nil, so `dew.overlay{}` without the word in the manifest
--- is a nil call at the line that wanted it.
declare dew: {
	--- Ungated. Reading a clock discloses nothing an applet could not get by
	--- counting frames, and gating it would put a permission in every manifest
	--- that exists to say "this applet is allowed to know what time it is".
	time: Clock,

	--- Requires "widget". A floating, borderless, always-on-top surface, which
	--- is what Dew is for.
	widget: ((WidgetOptions?) -> any)?,

	--- Requires "window". An ordinary window: caption, border, taskbar.
	window: ((WindowOptions?) -> any)?,

	--- Requires "overlay".
	overlay: ((OverlayOptions?) -> any)?,

	--- Requires "storage". Persists for the life of the host process; not yet
	--- written to disk.
	storage: Storage?,

	--- Requires "clipboard".
	clipboard: Clipboard?,

	--- Requires "notifications". Prints to the host's console until there is a
	--- tray to raise these through.
	notifications: Notifications?,

	--- Requires "audio".
	audio: Audio?,
}

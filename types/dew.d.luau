--!strict
--[[
	Type declarations for what a Dew applet is handed.

	Kept beside the host that produces it: `host/src/capabilities.rs` builds the
	table this describes, and `host/src/applets.rs` reads the declaration. If those
	two and this file disagree, they are wrong and this is stale.
]]

--- What an applet's module returns.
---
--- A DECLARATION, not a registration. Requiring the module defines this table
--- and does nothing else, so the host can read `id` and `size`, check them
--- against `dew.toml`, and refuse the applet before a line of its logic runs.
export type Mod = {
	id: string,
	--- Ignored for an overlay surface, which is sized by the screen.
	size: { width: number, height: number }?,
	surface: Surface?,

	--- Called ONCE, inside a reactive scope. Returns the tree, and optionally a
	--- table of transitions the host may drive from a hotkey.
	mount: (dew: Dew) -> (any, Actions?),
}

export type Actions = { [string]: () -> () }

--- Where the widget goes.
---
--- A tagged union rather than a bag of optional fields: `title` means nothing to
--- a floating widget and `anchor` means nothing to a window. Omitting `surface`
--- entirely gets a widget.
export type Surface = WidgetSurface | WindowSurface | OverlaySurface

export type Anchor = "top-left" | "top-right" | "bottom-left" | "bottom-right" | "center"

export type WidgetSurface = {
	kind: "widget",
	--- Defaults to "top-right". An anchor rather than coordinates, because a
	--- desktop is not one size.
	anchor: Anchor?,
	--- Pixels from the anchored corner. Defaults to 24, 24.
	offset: { x: number, y: number }?,
	--- Let clicks reach whatever is behind, everywhere. A widget with a control
	--- wants this false, which is the default.
	clickThrough: boolean?,
}

export type WindowSurface = {
	kind: "window",
	--- Defaults to the mod's id.
	title: string?,
}

--- A widget the size of the desktop.
---
--- Clicks fall through wherever nothing was painted, so the root frame must set
--- `BackgroundTransparency = 1` or the overlay becomes a screen-sized sheet of
--- glass that swallows every click on the machine.
export type OverlaySurface = {
	kind: "overlay",
	--- Above every other window. Defaults to true.
	topmost: boolean?,
	clickThrough: boolean?,
}

--- The capability table `mount` receives.
---
--- EVERY FIELD BUT `time` IS OPTIONAL, and present only if `dew.toml` declared
--- the matching permission. That is the point: what an applet cannot name it cannot
--- reach, and an applet declaring nothing gets a table carrying only the clock.
export type Dew = {
	--- Ungated. Reading a clock discloses nothing an applet could not get by counting
	--- frames, and gating it would put a permission in every manifest that exists
	--- to say "this mod is allowed to know what time it is".
	time: {
		--- Seconds since the epoch, as a float.
		now: () -> number,
	},

	--- Requires "storage". Persists for the life of the host process; not yet
	--- written to disk.
	storage: {
		get: (key: string) -> string?,
		set: (key: string, value: string) -> (),
	}?,

	--- Requires "clipboard".
	clipboard: {
		read: () -> string,
		write: (text: string) -> (),
	}?,

	--- Requires "notifications". Prints to the host's console until there is a
	--- tray to raise these through.
	notifications: {
		show: (message: string) -> (),
	}?,

	--- Requires "audio".
	audio: {
		chime: (name: string) -> (),
	}?,
}

return nil

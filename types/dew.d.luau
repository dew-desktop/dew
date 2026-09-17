--!strict
--[[
	What a Dew applet is handed, as the language server sees it.

	A DEFINITIONS FILE, not a module. It is loaded with `--definitions=` rather
	than required, which is what lets it declare a GLOBAL: `dew` is one, the way
	`game` is one in the engine this host is shaped after, and without this file
	an editor reports "Unknown global 'dew'" on every applet.

	Kept beside the host that produces it: `host/src/capabilities.rs` and
	`host/src/services.rs` both add members to the table this describes, and
	`host/src/applets.rs` installs it. If those three and this file disagree,
	they are wrong and this is stale.

	A surface returns `any` and should return `Instance`. Definition files are
	parsed one at a time and share no types with each other, so naming `Instance`
	here makes the whole file fail to load wherever the DataModel types are not in
	it, and an applet then gets no `dew` at all. This tightens when Dew ships type
	definitions for its own DataModel.
]]

type Anchor = "topLeft" | "topRight" | "bottomLeft" | "bottomRight" | "center"

type FloatOptions = {
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

--- A float the size of the desktop.
---
--- Clicks fall through wherever nothing was painted, so the root frame must set
--- `BackgroundTransparency = 1` or the overlay becomes a screen-sized sheet of
--- glass that swallows every click on the machine.
type OverlayOptions = {
	--- The screen supplies the size when this runs in a window. These are the
	--- size it was designed against, and what a headless render uses.
	width: number?,
	height: number?,
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

type Time = {
	--- Seconds since the epoch, as a float.
	now: () -> number,
}

--- A signal a host fires, shaped the way the engine shapes one.
type Signal = {
	Connect: (self: Signal, listener: (input: any) -> ()) -> { Disconnect: (self: any) -> () },
}

--- Frames, for anything that has to run per frame rather than per event.
---
--- NOT `Time`. `Time.now()` is wall-clock seconds since the epoch, the way an
--- applet would show the time of day; `Clock.Now()` is a monotonic reading
--- since the host's first frame, for animation to subtract two of. Same shape,
--- two different clocks, kept apart on purpose.
type Clock = {
	Name: string,
	OnFrame: (listener: (dt: number) -> ()) -> () -> (),
	Now: () -> number,
}

--- The capability table an applet is given.
---
--- `Time`, `Pointer`, `Input`, `Clock` and `Text` ARE NOT CAPABILITIES. Nothing
--- here is granted or withheld by a manifest: a clock, a pointer and text
--- metrics are what any host has, and a framework reaches for them the way it
--- reaches for the engine's own services. Every other field is OPTIONAL, and
--- present only if `dew.toml` declared the matching permission -- what an
--- applet cannot name it cannot reach, and one declaring nothing still gets a
--- table carrying the ungated members.
---
--- A SURFACE IS A FUNCTION HERE, not a block an applet returns. Calling it is
--- how the applet asks, and what comes back is the root to parent into. An
--- ungranted surface is nil, so `dew.Overlay{}` without the word in the manifest
--- is a nil call at the line that wanted it.
declare dew: {
	--- Ungated. Reading a clock discloses nothing an applet could not get by
	--- counting frames, and gating it would put a permission in every manifest
	--- that exists to say "this applet is allowed to know what time it is".
	Time: Time,

	--- Where the pointer is, asked of whichever host can answer. Ungated.
	Pointer: {
		Position: () -> (number?, number?),
		IsDown: (button: number?) -> boolean,
	},

	--- What the host delivers through, the way the engine delivers through
	--- `UserInputService`. A press and its release inside one frame cannot be
	--- recovered by polling, so they arrive here. Ungated.
	Input: {
		InputBegan: Signal,
		InputEnded: Signal,
		InputChanged: Signal,
	},

	--- Ungated.
	Clock: Clock,

	--- Measuring text, which needs the face the host will actually draw with.
	--- Ungated.
	Text: {
		Measure: (text: string, size: number, font: string?) -> (number, number),
	},

	--- Requires "float". A floating, borderless, always-on-top surface, which
	--- is what Dew is for.
	Float: ((FloatOptions?) -> any)?,

	--- Requires "window". An ordinary window: caption, border, taskbar.
	Window: ((WindowOptions?) -> any)?,

	--- Requires "overlay".
	Overlay: ((OverlayOptions?) -> any)?,

	--- Requires "storage". Persists for the life of the host process; not yet
	--- written to disk.
	Storage: Storage?,

	--- Requires "clipboard".
	Clipboard: Clipboard?,

	--- Requires "notifications". Prints to the host's console until there is a
	--- tray to raise these through.
	Notifications: Notifications?,

	--- Requires "audio".
	Audio: Audio?,
}

# The Dew applet contract

## An applet describes itself; it does not perform a registration

```luau
local Aether = require("@aether")
local create, source = Aether.create, Aether.source

local function mount(dew, root)
    local elapsed = source(0)
    ;(create "Frame" { --[[ ... ]] } :: any).Parent = root
end

return {
    id = "timetracker",
    size = { width = 380, height = 56 },
    mount = mount,
}
```

Loading an applet **describes** it. It does not **do** anything until the host
calls `mount`.

The previous shape was `dew.hud.registerComponent({ ... })` -- a call with
effects, made during load. That ordering makes the host's job impossible: to
find out what a mod is, it had to run the mod, and by then the applet had
already reached for whatever it wanted. A declaration can be read, checked
against `dew.toml`, and refused, all before a line of the mod's own logic
executes.

This is the same rule the runtime already enforces one layer down: the guest
VM is deny-by-default and every capability is a function the host installs by
name. A registration-by-side-effect API quietly undoes that at the layer
above.

## One loader, one signature, no framework named

`mount(dew, root)` is the only shape there is. The host gives every mod a root
to parent into and the `dew` capability table, and knows nothing else about
what the mod is built from:

```luau
return {
    id = "nameplate",
    size = { width = 340, height = 148 },

    mount = function(dew, root)
        local card = Instance.new("Frame")
        card.Size = UDim2.new(1, 0, 1, 0)
        card.BackgroundColor3 = Color3.fromRGB(20, 26, 36)
        card.Parent = root
    end,
}
```

Discovery, the manifest, the sandbox, the capability table, `size`, and the
fact that `mount` runs once are decided the same way for every applet. An
applet that wants a framework requires one from its own installed packages
(`@aether`, `@vide`, or anything else) and mounts it itself; an applet that
wants nothing writes `Instance.new` directly, the way `examples/widgets/nameplate`
does. Neither is a manifest field, because `mount(dew, root)` looks identical
either way -- what differs is what runs *inside* the function the host calls,
which is the applet's business and not the loader's.

This used to be two contracts: `mount(dew)` for an applet built with Aether,
called inside a reactive scope with a `Session` the host drove, and
`mount(dew, root)` for one built against the DataModel directly. Milestone 9
removed the first arm along with `Runtime`, the manifest field that chose
between them, because the difference it named was never the host's to
ceremony over. `UserInputService` and `RunService.Heartbeat` (ADR-010) are
real DataModel members now, so a framework that drives itself by polling
services -- which is what Aether already does on the engine -- drives itself
identically here. The host delivers input to nobody; it answers when asked.

**The root is a parameter, like `dew`.** No applet reaches for a global to
find where it draws. The root is a `ScreenGui` named `DewRoot` -- not `game`,
which nothing in Dew installs -- and it exists before `mount` is called,
whether the applet asked for its surface explicitly or is about to read one
off the return value below.

**A mod can be clicked.** Tree navigation (`GetChildren`, `FindFirstChild`,
`IsA`, `Destroy`), the signal model (`Changed`, `GetPropertyChangedSignal`,
the tree-and-lifecycle events, `:Connect`/`:Disconnect`), and the whole
pointer family (`Activated`, hover, hit testing) all reach a mod written with
no framework at all. Three things to know about them. **Assigning a property
the value it already has fires nothing** -- that is deliberate, and it is what
lets the host draw a static mod once instead of every frame. **`signal:Wait()`
is not available**: it yields on the engine and Dew has no task scheduler, so
it raises a message saying so rather than pretending; connect a handler. And
**a handler that errors is reported and does not fail the write that notified
it**, because on the engine each handler has its own thread and here it does
not.

## Surfaces are capabilities

`dew.Widget{}`, `dew.Window{}`, `dew.Overlay{}` and `dew.popover{}` each ask
the host for somewhere to draw and hand back the root to parent into --
**before `mount` is even called, if the applet wants**:

```luau
local root = dew.Widget({ width = 220, height = 56, anchor = "top-right" })
local frame = Instance.new("Frame")
frame.Size = UDim2.new(1, 0, 1, 0)
frame.Parent = root
```

Each is its own permission (`widget`, `window`, `overlay`, `popover`), because
they differ in weight. A widget draws in a corner. An overlay that is topmost
and click-through can draw over everything on screen while the user does not
know it is there. Granting those with one word would be saying they are the
same request. `dew.Overlay` is absent from the table unless `dew.toml` grants
`overlay`, and reaching for it anyway is a nil index at the applet's own call
site rather than a permission check somewhere else.

**Asking creates the surface immediately, mid-run, from inside a Luau call.**
That is what lets a popover open and close repeatedly over one applet's
lifetime -- `Combobox`, `ContextMenu` and `Tooltip` all work this way -- rather
than every surface having to exist before the applet's module even loads.

**An applet may also describe its surface by returning one**, instead of or
alongside asking:

```luau
return {
    id = "clock",
    size = { width = 220, height = 56 },
    surface = { kind = "widget", anchor = "top-right" },
    mount = function(dew, root) --[[ ... ]] end,
}
```

Omitting `surface` gets a widget. Dew is a desktop applet platform; a default
of "ordinary window" would make every author opt in to the thing they came
for. **What was asked for while the module ran wins over what it returned** --
an applet doing both is mid-migration rather than in conflict, and the call is
the newer of the two statements.

An applet that asked for its surface has already built its tree by the time it
returns, so there is nothing left to call: `mount` is optional in that case,
and required otherwise. An applet that does neither -- no ask, no `mount` -- is
refused at load, naming both ways out.

### A tagged union, not a bag of optional fields

`title` means nothing to a floating surface and `anchor` means nothing to a
window. Whether asked for as a call's options table or read from a returned
`surface` field, the shape is the same:

```luau
dew.Window({ title = "Time Tracker Settings" })
```

```luau
dew.Widget({
    anchor = "top-right",       -- "top-left" | "top-right" | "bottom-left" | "bottom-right" | "center"
    offset = { x = 24, y = 24 },
    clickThrough = false,
    zOrder = "topmost",          -- "bottom" | "normal" | "topmost", defaults to "topmost"
    draggable = true,            -- grab the body and move it; no title bar needed
    keepOnScreen = true,         -- default; a dragged widget cannot end up off every display
    snapToEdges = true,          -- snap to a screen edge while dragging
    savePosition = true,         -- a dragged position survives to the next launch
})
```

```luau
dew.Overlay({ zOrder = "topmost", clickThrough = false })
```

`zOrder` also applies to an overlay. `draggable`, `keepOnScreen`,
`snapToEdges` and `savePosition` are widget-only: dragging a screen-filling
overlay, or a window the desktop already lets you drag, makes no sense.
Dragging is entirely host-driven -- an applet's own code never sees the drag,
the same way it never sees the pump that delivers its pointer events.

### An overlay is a widget the size of the desktop

```luau
dew.Overlay({})   -- zOrder = "topmost", clickThrough = false
```

The screen supplies the size, so a size an applet asks for is ignored for this
kind -- a mod cannot know the display it will land on, and one that guessed
would be wrong on every machine but the author's.

**Clicks fall through wherever nothing was painted**, and that is a property
of layered windows rather than a trick on top of them: Windows hit-tests one
against its alpha channel, so a transparent pixel passes the click to whatever
is behind. An overlay that paints three cards is click-through everywhere
except those three cards, with no region and no hit-test hook.

Which makes one rule absolute: **the root frame must be
`BackgroundTransparency = 1`.** A filled backdrop turns the overlay into a
screen-sized sheet of glass that swallows every click on the machine.

### A widget's shape is its own alpha

On a widget surface the frame is cleared to NOTHING, so a pixel the tree did
not paint is a pixel the window does not occupy -- the desktop shows through
it and receives the click. A `UICorner` on the root frame is therefore the
window's real silhouette, not a rounded shape drawn on a dark rectangle.

`anchor` rather than raw coordinates because a desktop is not one size: a
clock pinned 24px from the top-right stays in the corner when the display
changes; one at `x = 1872` is in the corner of the display it was written on.

## Capabilities arrive as an argument, never as a global

`mount` receives `dew`. `dew.toml` declares `permissions`:

```json
"permissions": ["storage", "audio", "notifications"]
```

What an applet cannot name in `permissions`, it cannot reach: the capability
table is built from the granted permissions only, so an applet that quietly
used `clipboard` without declaring it gets a nil index at its own call site,
not silent success.

It also makes the boundary testable. "What can this mod do" is the table the
host built, which can be printed, diffed against the manifest, and asserted
on.

## `mount` runs once

It builds a tree; it is not a per-frame `render`. A `source` written from a
hotkey or a timer reaches the screen the way it does on the engine: the graph
re-runs what depends on it, and the next frame differs. The same component
behaves identically in both places, which is the property the whole stack
exists to preserve.

## An Aether applet authors in Aether's own idiom

`create`, `source`, `derive`, and the engine's own property vocabulary --
`UDim2`, `Color3`, `BackgroundTransparency`. Not a Dew dialect.

This is what keeps a widget's visual half **liftable**: the tree an applet
builds is an ordinary Aether component, so it can be mounted on a conforming
engine host unchanged, or previewed with `aether snapshot`. A Dew-specific
construction API would make every widget a dead end.

Dew's own additions are capabilities and lifecycle, not construction.

The same argument is why an applet with no framework authors in the engine's
own idiom rather than a host one: `Instance.new`, property assignment and
`Parent` are what an engine developer already knows, and
`examples/widgets/nameplate` builds the identical tree an Aether component
would, one `Instance.new` at a time. Neither is a Dew dialect; they are the
two idioms that already exist.

## A mod's images live beside it, under `mod://`

An `ImageLabel` or an `ImageButton` names an asset the way an engine
application does, with either generation of the property:

```luau
local icon = Instance.new("ImageLabel")
icon.Size = UDim2.new(0, 44, 0, 44)
icon.BackgroundTransparency = 1
icon.ImageContent = Content.fromUri("mod://droplet.png")   -- modern: a Content
-- icon.Image = "mod://droplet.png"                        -- legacy: a ContentId
icon.ScaleType = Enum.ScaleType.Fit
icon.Parent = root
```

`mod://` resolves against **the directory the applet was loaded from** -- the
same directory `require` is allowed to reach, and for the same reason. A path
that climbs out of it is refused rather than followed, so an image cannot
become the way around the boundary the requirer already enforces. A
`--script` run resolves `mod://` beside the script.

**Both properties name the same asset and Dew takes either.** `Image` is the
legacy `ContentId` string and `ImageContent` is the modern `Content` URI; a
mod written five years ago needs no editing to draw here. When a guest sets
both, `ImageContent` wins, because that is the one the engine's own migration
keeps.

### What is honoured

| | |
| :--- | :--- |
| `Image`, `ImageContent` | the asset, either generation |
| `ImageColor3` | multiplied through the asset's pixels, not replacing them |
| `ImageTransparency` | inverted into alpha, like every other transparency |
| `ImageRectOffset`, `ImageRectSize` | a sub-rectangle of the source, for sprite sheets |
| `Enum.ScaleType` | `Stretch`, `Fit` and `Crop` |

**`Enum.ScaleType.Slice` and `.Tile` are NOT drawn yet**, and with them
`SliceCenter`, `SliceScale` and `TileSize`. They are stretched instead, and
the host says so by name on the console the first time an element asks for
one. The properties still assign and still read back; what is missing is the
drawing, and you are told which of the two of you decided that.

### An asset that will not resolve is a missing image, not an error

`mod://` is the only scheme Dew resolves by default. `rbxassetid://` needs the
`rbxassetid` permission -- see ADR-003. Until granted, assigning one succeeds,
the property keeps its value, the host names the URI on the console once, and
the element is drawn as an empty marked box rather than as nothing at all.

That last part is deliberate. A blank space is indistinguishable from an
element that was never created, was positioned off-screen, or was made
invisible, and an author would check all three before suspecting the asset.
An engine application moved to Dew with an asset it cannot reach is a correct
application missing an image, not a broken one.

## Every `dew.toml` field, and whether the host reads it

A manifest field that is parsed and ignored is indistinguishable, from the
author's side, from one that works. So the list is exhaustive and the host
says the difference out loud at load.

| field | read by |
| :--- | :--- |
| `id` | the entry module `<id>.luau` |
| `permissions` | the capability table, and nothing outside this list is reachable |
| `name` | the window caption when the declaration sets no `surface.title`, and the tray tooltip |
| `description` | the tray tooltip |
| `hotkeys` | **nothing yet.** Declared and inert; see below |

`hotkeys` is the one field Dew accepts and does not act on. Nothing in the
host registers a global hotkey, so a declared binding has never fired.
`examples/aether/timetracker` ships three. Rather than delete the field, which
would make the format quietly narrower without deciding anything, the host
reports each one by name at load:

```
[dew] timetracker: hotkey "togglePomodoro" (Alt+Shift+P) is declared and not bound: Dew registers no global hotkeys yet
```

**Unknown keys are reported, not refused.** `timetracker` also carries
`version`, `author` and a `settings` block, and serde drops an unknown key
without a word, so all three went nowhere for as long as they existed. Each
now prints a line. They are not rejected because a manifest is a
forward-compatible format: a host that refuses tomorrow's field cannot read
tomorrow's mod. A `dew.toml` written before milestone 9 that still declares
`runtime` gets exactly this treatment -- reported once, at load, and
otherwise ignored -- because `mount(dew, root)` was always the only signature
that field ever selected.

The list that closes this out is `Manifest::unhonoured` in
`host/src/manifest.rs`. Empty is the goal, and a field that stops being read
cannot be added without appearing in it.

## What Dew owns, and what it does not

| | |
| :--- | :--- |
| **Aether** | layout, hit testing, pointer arbitration, focus, motion -- *for an applet that chose it* |
| **`crates/runtime`** | the VM, require resolution, the frame loop, the display list |
| **`crates/raster`** | the rasteriser |
| **`crates/window`** | the window, input, the blit |
| **Dew** | mod discovery, manifests, capabilities, hotkeys, tray, storage, multi-window placement, and its own DataModel |

**THE BOTTOM FOUR ROWS ARE ALL DEW.** They used to be one row and three
crates borrowed from Aether's repository, and this section said "when
something here needs a rendering change, it belongs upstream in Aether, where
the engine host gets it too". That was wrong, and it was the sentence that
made adding an image node to the display list look like changing a public
framework contract. It is a host editing its own renderer. ADR-004 measured
it and moved the crates; a rendering change belongs in `crates/`, and an
engine-side author never sees it because they never had it.

# The Dew applet contract

## An applet returns a declaration; it does not perform a registration

```luau
local Aether = require("@aether")
local create, source = Aether.create, Aether.source

return {
    id = "timetracker",
    size = { width = 380, height = 56 },

    mount = function(dew)
        local elapsed = source(0)
        return create "Frame" { --[[ ... ]] }
    end,
}
```

Loading an applet **describes** it. It does not **do** anything.

The previous shape was `dew.hud.registerComponent({ ... })` -- a call with effects,
made during load. That ordering makes the host's job impossible: to find out what
a mod is, it had to run the mod, and by then the applet had already reached for
whatever it wanted. A declaration can be read, checked against `dew.toml`, and
refused, all before a line of the mod's own logic executes.

This is the same rule the runtime already enforces one layer down: the guest VM
is deny-by-default and every capability is a function the host installs by name.
A registration-by-side-effect API quietly undoes that at the layer above.

## An applet declares the runtime it is written against

```json
{ "id": "nameplate", "runtime": "datamodel", "permissions": ["storage"] }
```

Two flavours, one loader. `runtime` is `"aether"` when absent, which is what every
mod written before the key existed is.

| | `"aether"` | `"datamodel"` |
| :--- | :--- | :--- |
| the applet imports | `@aether/api` | nothing |
| the VM gets | `Instance` | `Instance`, plus `UDim2`, `Color3`, `Vector2`, `UDim`, `Rect`, `Enum` and `Content` |
| `mount` is | `function(dew)`, returning a tree | `function(dew, root)`, parenting into `root` |
| it is called | through `Desktop.Mount`, inside a reactive scope | directly |
| the host drives | a `Session`, repainting when the graph changes | the DataModel arena, re-rendered every frame |

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

Everything else in this document is unchanged by the choice: discovery, the
manifest, the capability table, `size`, `surface`, and the fact that `mount` runs
once are Dew's remit, and none of them is a property of the framework an author
picked.

**Declared, never sniffed.** Both flavours build a tree, and there is no artefact
that reliably tells them apart -- a heuristic on `require("@aether/api")` reads
source to decide how to execute it, and answers wrong for the first mod that
requires the framework conditionally. And getting it wrong is not cosmetic: an
Aether mod on the DataModel branch never opens a reactive scope, and a DataModel
mod on the Aether branch is handed no root. So it is one key, in a closed set, and
`"runtime": "solid"` is refused at load rather than falling back.

**The root is a parameter, like `dew`.** A DataModel mod does not reach for a
global, because what an applet may draw into is granted to it in the same way as what
it may do. The root is a `ScreenGui` named `DewRoot` -- **not `game`**, which is
what `Host.detect()` keys on (`typeof(game) == "Instance"`); installing one before
the services and the member surface exist would flip every Aether mod in the same
binary onto its engine branch. That name arrives when there is enough behind it to
be true.

**No vocabulary for an Aether mod, deliberately.** Aether carries its own `UDim2`
and `Color3` for off-engine hosts and publishes them with
`if rawget(g, name) == nil` -- first writer wins -- so a partial host vocabulary
does not merge with Aether's, it *blocks* it. That the two branches install
different globals is the reason the runtime is declared, not an inconsistency
waiting to be tidied.

**A DataModel mod can navigate its tree and react to it, and cannot yet be
clicked.** `GetChildren`, `FindFirstChild`, `IsA`, `Destroy` and the rest of the
tree-and-lifecycle methods are there, and so is the signal model:
`RBXScriptSignal` and `RBXScriptConnection` with `:Connect` and `:Disconnect`,
`Changed`, `GetPropertyChangedSignal(name)`, `ChildAdded`, `ChildRemoved`,
`DescendantAdded`, `DescendantRemoving` and `Destroying`.

Three things to know about them. **Assigning a property the value it already has
fires nothing** -- that is deliberate, and it is what lets the host draw a static
mod once instead of every frame. **`signal:Wait()` is not available**: it yields
on the engine and Dew has no task scheduler, so it raises a message saying so
rather than pretending; connect a handler. And **a handler that errors is
reported and does not fail the write that notified it**, because on the engine
each handler has its own thread and here it does not.

There is still no click. Every event above is something an instance says about
ITSELF; `Activated` and the pointer events need a hit test, so pointer events are
still dropped for this flavour.

## Capabilities arrive as an argument, never as a global

`mount` receives `dew`. There is no `_G.dew`, and mods are not handed one.

`dew.toml` has always declared `permissions`:

```json
"permissions": ["storage", "audio", "notifications"]
```

While `dew` was a global, that list was decoration -- the whole surface was
reachable whatever the manifest said, and an applet that quietly used `clipboard`
without declaring it worked exactly as well as one that declared it. Passing the
capability table in makes the manifest load-bearing: what an applet cannot name, it
cannot reach.

It also makes the boundary testable. "What can this mod do" is the table the host
built, which can be printed, diffed against the manifest, and asserted on.

## `mount` runs once

It returns a tree; it is not a per-frame `render`.

The old `render` was called every frame and rebuilt the tree from scratch. Mods
declared `source` and `derive` correctly and then discarded the result each
frame, so the reactive graph was decorative and the rebuild did the work. That is
slower, and it diverges from the reference implementation -- where a mounted tree persists and the graph
drives property updates.

Under `mount`, a `source` written from a hotkey or a timer reaches the screen the
same way it does in an engine: the graph re-runs what depends on it, and the next
`Live.Frame` differs. The same component behaves identically in both places,
which is the property the whole stack exists to preserve.

## An Aether applet authors in Aether's own idiom

`create`, `source`, `derive`, and the reference implementation's property vocabulary -- `UDim2`,
`Color3`, `BackgroundTransparency`. Not a Dew dialect.

This is what keeps a widget's visual half **liftable**: the tree an applet builds is
an ordinary Aether component, so it can be mounted on a conforming engine host unchanged,
or previewed with `aether snapshot`. A Dew-specific construction API would make
every widget a dead end.

Dew's own additions are capabilities and lifecycle, not construction.

The same argument is why a `"datamodel"` applet authors in the engine's idiom rather
than a host one: `Instance.new`, property assignment and `Parent` are what a
an engine developer already knows, and `examples/widgets/nameplate` would build the identical
tree inside a place. Neither flavour is a Dew dialect; they are the two idioms
that already exist.

## One rendering path, with a shorthand above it

Two authoring scopes existed: `registerComponent` took an Aether tree, and
`registerWidget` took a data descriptor (`{ text, subtext, icon, color }`) that
the host rendered in a house style.

Both are worth having -- the shorthand is most of what a small widget wants -- but
they must not be two paths through the renderer. `dew.hud.card { ... }` is a Luau
helper that BUILDS an Aether tree, so the shorthand is a library function and the
engine has one path to keep correct.

## A mod's images live beside it, under `mod://`

An `ImageLabel` or an `ImageButton` names an asset the way an engine application
does, with either generation of the property:

```luau
local icon = Instance.new("ImageLabel")
icon.Size = UDim2.new(0, 44, 0, 44)
icon.BackgroundTransparency = 1
icon.ImageContent = Content.fromUri("mod://droplet.png")   -- modern: a Content
-- icon.Image = "mod://droplet.png"                        -- legacy: a ContentId
icon.ScaleType = Enum.ScaleType.Fit
icon.Parent = root
```

`mod://` resolves against **the directory the applet was loaded from** -- the same
directory `require` is allowed to reach, and for the same reason. A path that
climbs out of it is refused rather than followed, so an image cannot become the
way around the boundary the requirer already enforces. A `--script` run resolves
`mod://` beside the script.

**Both properties name the same asset and Dew takes either.** `Image` is the
legacy `ContentId` string and `ImageContent` is the modern `Content` URI; a mod
written five years ago needs no editing to draw here. When a guest sets both,
`ImageContent` wins, because that is the one the engine's own migration keeps.

### What is honoured

| | |
| :--- | :--- |
| `Image`, `ImageContent` | the asset, either generation |
| `ImageColor3` | multiplied through the asset's pixels, not replacing them |
| `ImageTransparency` | inverted into alpha, like every other transparency |
| `ImageRectOffset`, `ImageRectSize` | a sub-rectangle of the source, for sprite sheets |
| `Enum.ScaleType` | `Stretch`, `Fit` and `Crop` |

**`Enum.ScaleType.Slice` and `.Tile` are NOT drawn yet**, and with them
`SliceCenter`, `SliceScale` and `TileSize`. They are stretched instead, and the
host says so by name on the console the first time an element asks for one. The
properties still assign and still read back; what is missing is the drawing, and
you are told which of the two of you decided that.

### An asset that will not resolve is a missing image, not an error

`mod://` is the only scheme Dew resolves today. `rbxassetid://` needs a scheme
registry, a permission and a fetch, and those arrive together -- see ADR-003.
Until then, assigning one succeeds, the property keeps its value, the host names
the URI on the console once, and the element is drawn as an empty marked box
rather than as nothing at all.

That last part is deliberate. A blank space is indistinguishable from an element
that was never created, was positioned off-screen, or was made invisible, and an
author would check all three before suspecting the asset. An engine application
moved to Dew with an asset it cannot reach is a correct application missing an
image, not a broken one.

## Widgets and windows are one API, not two

A mod declares the surface it wants beside its size:

```luau
surface = {
    kind = "widget",              -- floating, no chrome, on the desktop
    anchor = "top-right",
    offset = { x = 24, y = 24 },
    clickThrough = false,
}
```

```luau
surface = { kind = "window", title = "Time Tracker Settings" }
```

**One `mount`, because the applet builds the same tree either way.** Chrome or none,
in the taskbar or not, blitted into a rectangle or composited from its own alpha
-- every one of those is a property of the WINDOW, not of the widget. Two entry
points would mean two paths through the loader for a difference that is entirely
window-creation flags, and would force an applet wanting both a HUD and a settings
panel to be two mods.

**A tagged union, though, not a bag of optional fields.** `title` means nothing
to a floating widget and `anchor` means nothing to a window. A flat table would
let an applet set either and have it silently ignored, which is exactly how
`permissions` was decoration before it was enforced.

Omitting `surface` gets a widget. Dew is a desktop applet platform; a default of
"ordinary window" would make every author opt in to the thing they came for.

### An overlay is a widget the size of the desktop

```luau
surface = { kind = "overlay" }   -- topmost = true, clickThrough = false
```

The screen supplies the size, so `size` in the declaration is ignored -- a mod
cannot know the display it will land on, and one that guessed would be wrong on
every machine but the author's.

**Clicks fall through wherever nothing was painted**, and that is a property of
layered windows rather than a trick on top of them: Windows hit-tests one against
its alpha channel, so a transparent pixel passes the click to whatever is behind.
An overlay that paints three cards is click-through everywhere except those three
cards, with no region and no hit-test hook.

Which makes one rule absolute: **the root frame must be
`BackgroundTransparency = 1`.** A filled backdrop turns the overlay into a
screen-sized sheet of glass that swallows every click on the machine.

### A widget's shape is its own alpha

On a widget surface the frame is cleared to NOTHING, so a pixel the tree did not
paint is a pixel the window does not occupy -- the desktop shows through it and
receives the click. A `UICorner` on the root frame is therefore the window's real
silhouette, not a rounded shape drawn on a dark rectangle.

`anchor` rather than raw coordinates because a desktop is not one size: a clock
pinned 24px from the top-right stays in the corner when the display changes; one
at `x = 1872` is in the corner of the display it was written on.

## Every `dew.toml` field, and whether the host reads it

A manifest field that is parsed and ignored is indistinguishable, from the
author's side, from one that works. So the list is exhaustive and the host says
the difference out loud at load.

| field | read by |
| :--- | :--- |
| `id` | mod selection (`--mod`), and the entry module `<id>.luau` |
| `runtime` | which branch `applets::load` takes: `"aether"` (the default) or `"datamodel"` |
| `permissions` | the capability table, and nothing outside this list is reachable |
| `name` | the window caption when the declaration sets no `surface.title`, and the tray tooltip |
| `description` | the tray tooltip |
| `hotkeys` | **nothing yet.** Declared and inert; see below |

`hotkeys` is the one field Dew accepts and does not act on. Nothing in the host
registers a global hotkey, so a declared binding has never fired.
`examples/aether/timetracker` ships three. Rather than delete the field, which would make
the format quietly narrower without deciding anything, the host reports each one
by name at load:

```
[dew] timetracker: hotkey "togglePomodoro" (Alt+Shift+P) is declared and not bound: Dew registers no global hotkeys yet
```

**Unknown keys are reported, not refused.** `timetracker` also carries `version`,
`author` and a `settings` block, and serde drops an unknown key without a word,
so all three went nowhere for as long as they existed. Each now prints a line.
They are not rejected because a manifest is a forward-compatible format: a host
that refuses tomorrow's field cannot read tomorrow's mod.

The list that closes this out is `Manifest::unhonoured` in `host/src/manifest.rs`.
Empty is the goal, and a field that stops being read cannot be added without
appearing in it.

## What Dew owns, and what it does not

| | |
| :--- | :--- |
| **Aether** | layout, hit testing, pointer arbitration, focus, motion -- *for an applet that chose it* |
| **`crates/runtime`** | the VM, require resolution, the frame loop, the display list |
| **`crates/raster`** | the rasteriser |
| **`crates/window`** | the window, input, the blit |
| **Dew** | mod discovery, manifests, capabilities, hotkeys, tray, storage, multi-window placement, and its own DataModel |

**THE BOTTOM FOUR ROWS ARE ALL DEW.** They used to be one row and three crates
borrowed from Aether's repository, and this section said "when something here
needs a rendering change, it belongs upstream in Aether, where the engine host
gets it too". That was wrong, and it was the sentence that made adding an image
node to the display list look like changing a public framework contract. It is a
host editing its own renderer. ADR-004 measured it and moved the crates; a
rendering change belongs in `crates/`, and an engine-side author never sees it because
they never had it.

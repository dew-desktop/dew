# The Dew mod contract

## A mod returns a declaration; it does not perform a registration

```luau
local Aether = require("@aether")
local create, source = Aether.create, Aether.source

return {
    id = "timetracker",
    size = { width = 380, height = 56 },

    mount = function(dew)
        local elapsed = source(0)
        return create "Frame" { --[[ … ]] }
    end,
}
```

Loading a mod **describes** it. It does not **do** anything.

The previous shape was `dew.hud.registerComponent({ … })` — a call with effects,
made during load. That ordering makes the host's job impossible: to find out what
a mod is, it had to run the mod, and by then the mod had already reached for
whatever it wanted. A declaration can be read, checked against `mod.json`, and
refused, all before a line of the mod's own logic executes.

This is the same rule the runtime already enforces one layer down: the guest VM
is deny-by-default and every capability is a function the host installs by name.
A registration-by-side-effect API quietly undoes that at the layer above.

## Capabilities arrive as an argument, never as a global

`mount` receives `dew`. There is no `_G.dew`, and mods are not handed one.

`mod.json` has always declared `permissions`:

```json
"permissions": ["storage", "audio", "notifications"]
```

While `dew` was a global, that list was decoration — the whole surface was
reachable whatever the manifest said, and a mod that quietly used `clipboard`
without declaring it worked exactly as well as one that declared it. Passing the
capability table in makes the manifest load-bearing: what a mod cannot name, it
cannot reach.

It also makes the boundary testable. "What can this mod do" is the table the host
built, which can be printed, diffed against the manifest, and asserted on.

## `mount` runs once

It returns a tree; it is not a per-frame `render`.

The old `render` was called every frame and rebuilt the tree from scratch. Mods
declared `source` and `derive` correctly and then discarded the result each
frame, so the reactive graph was decorative and the rebuild did the work. That is
slower, and it diverges from Roblox — where a mounted tree persists and the graph
drives property updates.

Under `mount`, a `source` written from a hotkey or a timer reaches the screen the
same way it does in an engine: the graph re-runs what depends on it, and the next
`Live.Frame` differs. The same component behaves identically in both places,
which is the property the whole stack exists to preserve.

## Mods author in Aether's own idiom

`create`, `source`, `derive`, and Roblox's property vocabulary — `UDim2`,
`Color3`, `BackgroundTransparency`. Not a Dew dialect.

This is what keeps a widget's visual half **liftable**: the tree a mod builds is
an ordinary Aether component, so it can be mounted in a Roblox place unchanged,
or previewed with `aether snapshot`. A Dew-specific construction API would make
every widget a dead end.

Dew's own additions are capabilities and lifecycle, not construction.

## One rendering path, with a shorthand above it

Two authoring scopes existed: `registerComponent` took an Aether tree, and
`registerWidget` took a data descriptor (`{ text, subtext, icon, color }`) that
the host rendered in a house style.

Both are worth having — the shorthand is most of what a small widget wants — but
they must not be two paths through the renderer. `dew.hud.card { … }` is a Luau
helper that BUILDS an Aether tree, so the shorthand is a library function and the
engine has one path to keep correct.

## What Dew owns, and what it does not

| | |
| :--- | :--- |
| **Aether** | layout, hit testing, pointer arbitration, focus, motion |
| **`aether_runtime`** | the VM, require resolution, the frame loop, the display list |
| **`aether_window`** | the window, input, the blit |
| **Dew** | mod discovery, manifests, capabilities, hotkeys, tray, storage, multi-window placement |

Dew contains no layout code and no drawing code. When something here needs a
rendering change, it belongs upstream in Aether, where the Roblox host gets it
too.

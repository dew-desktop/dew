---
title: Applet structure
description: The files that make up an applet, and how Dew uses them.
sidebar:
  order: 1
---

An **applet** is an app that people install on their computer and that runs on
the desktop, in a widget, a window or an overlay. If you have built a browser
extension or a progressive web app, applets will feel familiar: a manifest
describes the app and the permissions it needs, and people install it from a
package.

An applet is a directory with a `dew.toml` manifest and a Luau entry module in
it. Dew reads the manifest, loads the module, opens a surface for it to draw into,
and calls its `mount` function. This page describes each of those steps.

## Directory layout

```tree
clock/
  dew.toml       # what the applet is, and what it can reach
  clock.luau     # the entry module, named after the manifest's `id`
  droplet.png    # an image, loaded as `mod://droplet.png`
```

Dew loads the entry module named after the `id` in `dew.toml`, here `clock.luau`.
If there is no such file, it loads `main.luau` instead.

## Entry module

The entry module returns a table that describes your applet. Loading the module
only describes it; your interface is built when Dew calls `mount`.

```luau
return {
    size = { width = 340, height = 148 },

    mount = function(desktop, root)
        local card = Instance.new("Frame")
        card.Size = UDim2.new(1, 0, 1, 0)
        card.BackgroundColor3 = Color3.fromRGB(20, 26, 36)
        card.Parent = root
    end,
}
```

| Field | Type | Meaning |
| :--- | :--- | :--- |
| `size` | `{ width, height }` | The size of the surface, in pixels. Defaults to 380 by 56. |
| `surface` | table | Which kind of surface to open, and its options. Defaults to a widget. See [Surfaces](surfaces.md). |
| `mount` | `function(desktop, root)` | Builds your interface. Required unless the module asks for its surface while loading. |

### The `desktop` table

`desktop` holds everything your applet can reach outside its own interface, such
as the clock, storage or the clipboard. It contains only what your manifest
grants. Something you did not ask for is `nil`, so a missing permission shows up
as an error on the line that uses it.

`desktop` is available as a global while your module loads, and Dew passes the
same table to `mount`. The [`desktop` reference](desktop.md) lists every member.

### The root

`root` is the instance you parent your interface to. It is a `ScreenGui` named
`DewRoot`, and it already exists when `mount` runs. There is no `game` object in
Dew.

## Mounting

Dew calls `mount` once. Build your interface there, and update it by changing
properties later, from a timer, an event or a frame callback. You do not redraw
anything yourself; Dew draws whatever changed on the next frame.

- **Surface requested.** Your module builds its interface as it loads, so
  `mount` is optional.
- **No surface requested.** `mount` is required.
- **Neither.** Dew refuses to load the applet and tells you both ways to fix it.

## Permissions

List what your applet needs in `dew.toml`:

```toml
permissions = ["widget", "storage"]
```

Dew builds the `desktop` table from this list and nothing else, so ask for what
your applet uses and no more. The [manifest reference](manifest.md#permissions)
lists every permission.

## Frameworks

You can build your interface with `Instance.new` directly, or with a framework
such as Aether or vide. Install the framework into your applet's directory with
`pesde`, require it, and mount it inside `mount`:

```luau
local Aether = require("@aether")
local create, source = Aether.create, Aether.source

return {
    size = { width = 380, height = 56 },
    mount = function(desktop, root)
        local elapsed = source(0)
        ;(create "Frame" { --[[ ... ]] } :: any).Parent = root
    end,
}
```

Nothing in `dew.toml` changes either way. The [Interface](../interface/classes.md)
pages cover the classes, events and images you can use.

---
title: Applet structure
description: The files that make up an applet, and how Dew uses them.
sidebar:
  order: 1
---

An **applet** is a directory with a `dew.toml` manifest and a Luau entry module in
it. Dew reads the manifest, loads the module, opens a surface for it to draw into,
and calls its `mount` function. This page describes each of those steps, so you
know what to put in each file and what to expect when your applet runs.

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
| `surface` | table | Which kind of surface to open, and its options. Defaults to a widget. See [Surfaces](#surfaces). |
| `mount` | `function(desktop, root)` | Builds your interface. Required unless the module asks for its surface while loading. |

### The `desktop` table

`desktop` holds everything your applet can reach outside its own interface, such
as the clock, storage or the clipboard. It contains only what your manifest
grants. Something you did not ask for is `nil`, so a missing permission shows up
as an error on the line that uses it.

`desktop` is available as a global while your module loads, and Dew passes the
same table to `mount`. The [`desktop` reference](../reference/desktop.md) lists
every member.

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

## Surfaces

A **surface** is where your applet draws. Each kind needs its own permission in
`dew.toml`:

| Permission | Function | Opens |
| :--- | :--- | :--- |
| `widget` | `desktop.Widget` | A borderless panel on the desktop |
| `window` | `desktop.Window` | An ordinary window with a title bar |
| `overlay` | `desktop.Overlay` | A layer over the whole desktop |
| `popover` | `desktop.popover` | Reserved for a future kind of surface |

### Ask for a surface

Call the function, then parent your interface to the root it returns:

```luau
local root = desktop.Widget({ width = 220, height = 56, anchor = "top-right" })
local frame = Instance.new("Frame")
frame.Size = UDim2.new(1, 0, 1, 0)
frame.Parent = root
```

Because the root comes back straight away, you can build your whole interface at
the top level of the module. If you call more than one surface function, the
last call wins.

### Describe a surface

Alternatively, return the surface as `surface`. Without one, Dew opens a widget.

```luau
return {
    size = { width = 220, height = 56 },
    surface = { kind = "widget", anchor = "top-right" },
    mount = function(desktop, root) end,
}
```

If your module both asks for a surface and returns one, the one it asked for wins.

### Options

Each kind of surface takes its own options, whether you pass them to the function
or return them as `surface`. A window takes a title:

```luau
desktop.Window({ title = "Time Tracker Settings" })
```

A widget takes a position and how it responds to dragging:

```luau
desktop.Widget({
    anchor = "top-right",   -- "top-left" | "top-right" | "bottom-left" | "bottom-right" | "center"
    offset = { x = 24, y = 24 },
    clickThrough = false,
    zOrder = "topmost",     -- "bottom" | "normal" | "topmost"; defaults to "topmost"
    draggable = true,       -- drag the widget by its body
    keepOnScreen = true,    -- the default; a dragged widget stays on a display
    snapToEdges = true,     -- snap to a screen edge while dragging
    savePosition = true,    -- remember where the widget was dragged
})
```

Anchoring to a corner keeps a widget in that corner on any display size. The drag
options apply to widgets only, and Dew handles the dragging for you.

An overlay takes `zOrder` and `clickThrough`:

```luau
desktop.Overlay({ zOrder = "topmost", clickThrough = false })
```

### Overlays

An overlay always covers the whole desktop, so its size comes from the screen.

Clicks pass through every part of an overlay you do not paint. An overlay that
draws three cards responds to clicks on those cards, and everywhere else the
click reaches whatever is underneath.

> [!CAUTION]
> Give the root frame of an overlay `BackgroundTransparency = 1`. A filled
> background covers the whole desktop and catches every click on it.

### Widgets

Unpainted parts of a widget are fully transparent, and clicks on them reach the
desktop behind. A `UICorner` on the root frame therefore gives the widget rounded
corners that are real, not drawn on a dark rectangle.

## Permissions

List what your applet needs in `dew.toml`:

```toml
permissions = ["storage", "audio", "notifications"]
```

Dew builds the `desktop` table from this list and nothing else, so ask for what
your applet uses and no more. The [manifest reference](../reference/manifest.md) lists every
permission.

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

Nothing in `dew.toml` changes either way.

## Events and signals

Without a framework, you still have the instance tree and events you know from
Roblox:

- Navigation, such as `GetChildren`, `FindFirstChild`, `IsA` and `Destroy`
- Change signals, such as `Changed` and `GetPropertyChangedSignal`
- Pointer events, such as `Activated` and hover

Three things work differently from Roblox:

- **Unchanged values.** Setting a property to the value it already has fires no
  event.
- **No `Wait`.** `signal:Wait()` raises an error. Connect a handler instead.
- **Handler errors.** If a handler errors, Dew reports the error and the property
  change that fired it still takes effect.

[Supported classes](../reference/classes.md) lists every property, method and
event you can use.

## Images

An `ImageLabel` or `ImageButton` names its image with `Image` or `ImageContent`.
Use `mod://` followed by a path inside your applet's directory:

```luau
local icon = Instance.new("ImageLabel")
icon.Size = UDim2.new(0, 44, 0, 44)
icon.BackgroundTransparency = 1
icon.ImageContent = Content.fromUri("mod://droplet.png")
icon.ScaleType = Enum.ScaleType.Fit
icon.Parent = root
```

`icon.Image = "mod://droplet.png"` works too. If you set both, `ImageContent` wins.
A path that leads outside your applet's directory is refused.

| Property | Behavior |
| :--- | :--- |
| `Image`, `ImageContent` | The image to draw |
| `ImageColor3` | Tints the image |
| `ImageTransparency` | Fades the image |
| `ImageRectOffset`, `ImageRectSize` | Draws part of the image, for sprite sheets |
| `ScaleType` | `Stretch`, `Fit` and `Crop` |

`Slice` and `Tile` scaling are not drawn yet, and neither are `SliceCenter`,
`SliceScale` and `TileSize`. Images that ask for them are stretched, and Dew
prints a message the first time.

### Missing images

Dew loads `mod://` images by default. An `rbxassetid://` image needs the
`rbxassetid` permission; without it, Dew prints the address once. An image Dew
cannot load draws as an empty, marked box, which is easier to spot than empty
space.

## Experimental features

Some DataModel features are available before their design is final. To use one,
name it in `dew.toml` together with the version you wrote your applet against:

```toml
experimentalDatamodel = ["GuiObject.BlendingMode@1"]
```

The `@1` is required. If the feature changes in a later version of Dew, your
applet stops loading with a message that says what to update, instead of quietly
behaving differently.

When your applet loads, Dew prints each feature it turned on:

```text
[dew] experimental: FFlagDewGuiObjectBlendingMode (revision 1)
```

### Trying a feature locally

To turn a feature on for every applet on your computer without editing a
manifest, add its flag to `DewAppSettings.json` in Dew's settings folder. On
Windows, that is `%LOCALAPPDATA%\Dew\DewAppSettings.json`:

```json
{ "FFlagDewGuiObjectBlendingMode": true }
```

## Manifest fields

| Field | What it does |
| :--- | :--- |
| `id` | Names the entry module, `<id>.luau` |
| `name` | The window title, unless the surface sets its own, and the tray tooltip |
| `description` | The tray tooltip |
| `permissions` | What the applet can reach |
| `hotkeys` | Not active yet; each binding is reported as not bound |
| `experimentalDatamodel` | The experimental features the applet uses |

If `dew.toml` contains a field Dew does not recognize, Dew prints a message and
loads the applet anyway.

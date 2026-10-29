---
title: Surfaces
description: Widgets, windows and overlays, and how to open each one.
sidebar:
  order: 2
---

A **surface** is where your applet draws. Each kind needs its own permission in
`dew.toml`:

| Permission | Function | Opens |
| :--- | :--- | :--- |
| `widget` | `desktop.Widget` | A borderless panel on the desktop |
| `window` | `desktop.Window` | An ordinary window with a title bar |
| `overlay` | `desktop.Overlay` | A layer over the whole desktop |
| `popover` | `desktop.popover` | Reserved for a future kind of surface |

There are two ways to get a surface: ask for one while your module loads, or
describe one in the table your module returns.

## Ask for a surface

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

## Describe a surface

Alternatively, return the surface as `surface`. Without one, Dew opens a widget.

```luau
return {
    size = { width = 220, height = 56 },
    surface = { kind = "widget", anchor = "top-right" },
    mount = function(desktop, root) end,
}
```

If your module both asks for a surface and returns one, the one it asked for wins.

## Options

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

The [`desktop` reference](desktop.md#surfaces) lists every option with its
default.

## Widgets

Unpainted parts of a widget are fully transparent, and clicks on them reach the
desktop behind. A `UICorner` on the root frame therefore gives the widget rounded
corners that are real, not drawn on a dark rectangle.

## Overlays

An overlay always covers the whole desktop, so its size comes from the screen.

Clicks pass through every part of an overlay you do not paint. An overlay that
draws three cards responds to clicks on those cards, and everywhere else the
click reaches whatever is underneath.

> [!CAUTION]
> Give the root frame of an overlay `BackgroundTransparency = 1`. A filled
> background covers the whole desktop and catches every click on it.

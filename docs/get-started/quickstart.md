---
title: Quickstart
description: Build a widget that shows the time.
steps: true
sidebar:
  order: 1
---

An applet is a directory with two files in it: a manifest that says what the applet
is and what it can reach, and a Luau module that draws it. This page builds one
that shows the time in a corner of the desktop.

```tree
clock/
  dew.toml      # what the applet is, and what it can reach
  clock.luau    # the entry module, named after the manifest's `id`
```

## Create the directory

Make a directory called `clock`. Everything your applet needs goes inside it, and
your applet can read nothing outside it.

## Describe the applet `dew.toml`

Create `dew.toml` in the directory:

```toml title="clock/dew.toml"
id = "clock"
name = "Clock"
description = "The time, in a corner of the desktop"
permissions = ["widget"]
```

- `id` names the entry module, so Dew loads `clock.luau`.
- `name` is the title the tray shows.
- `permissions` lists what the applet can reach. `widget` lets it open a
  borderless surface on the desktop, and nothing else.

## Draw the time `clock.luau`

Create `clock.luau` beside the manifest:

```luau title="clock/clock.luau"
--!strict

local function mount(desktop, root: Instance)
	local label = Instance.new("TextLabel")
	label.Size = UDim2.fromScale(1, 1)
	label.BackgroundColor3 = Color3.fromRGB(18, 20, 26)
	label.TextColor3 = Color3.fromRGB(233, 239, 249)
	label.TextSize = 24
	label.Parent = root

	local corner = Instance.new("UICorner")
	corner.CornerRadius = UDim.new(0, 8)
	corner.Parent = label

	desktop.Clock.OnFrame(function()
		local t = desktop.Time.local_time()
		label.Text = string.format("%02d:%02d:%02d", t.hour, t.min, t.sec)
	end)
end

return {
	size = { width = 180, height = 56 },
	surface = { kind = "widget", anchor = "top-right" },
	mount = mount,
}
```

The module returns a table that describes your applet. Dew opens a widget in the
top-right corner at the size you asked for, then calls `mount` once with the
`desktop` table and the root to build your interface under.

- `Instance.new`, `UDim2` and `Color3` work as they do in Roblox.
- `desktop.Time` reads the wall clock, and `desktop.Clock.OnFrame` calls the
  function once per frame. Neither needs a permission.
- Setting `Text` to the value it already has does nothing, so the label redraws
  only when the second changes.

## Run the applet

Dew runs an applet from its directory. Today, the development build of the `dew`
command does this on Windows:

```sh
dew run clock
```

The time appears in the top-right corner of your screen. To check the applet
without opening a window, validate it and render one frame to an image:

```sh
dew check clock
dew snapshot clock -o clock.png
```

## Next steps

- [Applet structure](../spec/applet_contract.md) covers every field the module
  can return, the kinds of surface, and how images load.
- [Manifest reference](../reference/manifest.md) lists every field and permission
  in `dew.toml`.

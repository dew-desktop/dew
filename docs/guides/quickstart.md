---
title: Quickstart
description: Create and run your first Dew desktop applet in under 60 seconds.
---

import { Tabs, TabItem } from '@astrojs/starlight/components';

Dew applets are self-contained desktop widgets and background daemons powered by sandboxed Luau, Vello/DirectComposition rendering, and reactive state.

---

## Prerequisites

Before building an applet, ensure you have the Dew CLI installed:

```bash title="Terminal"
# Check your Dew engine installation
dew --version
```
Expected output format: `0.1.0.<build_number>` (or `0.1.0-dev.<sha>` for dev builds).

---

## 1. Scaffold an Applet

Use the `dew init` command to generate an applet scaffold:

```bash title="Terminal"
dew init ./my-clock
cd my-clock
```

This generates the minimal anatomy of a Dew applet:
```txt title="Directory Structure"
my-clock/
├── dew.toml      # Applet manifest: capabilities, permissions & metadata
└── main.luau     # Applet lifecycle entry point
```

---

## 2. Inspect the Manifest (`dew.toml`)

Open `dew.toml`. Dew operates on a **least-privilege capability security model**. If your applet needs to spawn a window, it must declare the `desktop.Widget` capability:

```toml title="dew.toml"
id = "clock"
name = "Desktop Clock"
description = "A floating clock applet"
permissions = ["widget"]
```

---

## 3. The Entry Point

Mount a simple UI using Dew's global `desktop` table and DataModel primitives:

```luau title="main.luau"
--!strict
local widget = assert(desktop.Widget, "dew.toml must grant `widget`")

-- Open the floating desktop surface
local root = widget({
    width = 240,
    height = 90,
    anchor = "top-right",
    offset = { x = 24, y = 24 },
    draggable = true,
})

-- Build UI primitives over the DataModel
local frame = Instance.new("Frame")
frame.Size = UDim2.fromScale(1, 1)
frame.BackgroundColor3 = Color3.fromRGB(24, 24, 27)
frame.Parent = root

local corner = Instance.new("UICorner")
corner.CornerRadius = UDim.new(0, 8)
corner.Parent = frame

local label = Instance.new("TextLabel")
label.Size = UDim2.fromScale(1, 1)
label.BackgroundTransparency = 1
label.TextColor3 = Color3.fromRGB(244, 244, 245)
label.TextSize = 24
label.Text = os.date("%H:%M:%S")
label.Parent = frame
```

---

## 4. Run with Hot Reloading

Start your applet:

```bash title="Terminal"
dew run ./my-clock
```

Dew launches your widget with instant GPU presentation and watches the folder for changes. Whenever you edit `init.luau` or change colors in your editor, the engine hot-reloads your state and UI in milliseconds without restarting the host process.

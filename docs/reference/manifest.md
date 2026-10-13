---
title: Manifest Specification (dew.toml)
description: Complete reference for the dew.toml applet manifest and permission scopes.
---

Every Dew applet must contain a `dew.toml` file at its root. Dew parses and validates the manifest **before** executing any guest code. Unknown keys or ungranted permissions are reported explicitly rather than failing silently.

---

## Minimal Example

```toml
id = "clock"
name = "Desktop Clock"
description = "A floating desktop clock widget"
permissions = ["widget"]
```

---

## Manifest Fields

| Field | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `id` | `string` | **Yes** | Unique identifier for the applet (e.g. `"clock"`, `"quickpanel"`). |
| `name` | `string` | No | Display title for the window header and tray tooltip. |
| `description` | `string` | No | Short summary shown in the dashboard and tray. |
| `permissions` | `list[string]` | No | Capabilities granted to the guest script. Defaults to `[]` (pure rendering, zero host access). |
| `hotkeys` | `table` | No | Key combinations reserved for the applet (e.g. `toggle = "Alt+Shift+P"`). |

---

## Permissions & Capabilities

Permissions in `dew.toml` are categorized into two host-managed capability tiers:

### 1. User Permissions (Ungated for Installed Applets)
Any installed applet may request these permissions in `permissions = [...]`:

| Permission | Injected Global | Purpose |
| :--- | :--- | :--- |
| `"widget"` | `desktop.Widget` | Opens a borderless floating surface. |
| `"window"` | `desktop.Window` | Opens a native window with title bar and borders. |
| `"overlay"` | `desktop.Overlay` | Opens a full-screen, click-through desktop overlay. |
| `"popover"` | `desktop.Popover` | Opens an anchored context popover. |
| `"storage"` | `desktop.Storage` | Sandboxed key-value state persistence. |
| `"system"` | `desktop.System` | System resource metrics (CPU, RAM, disk, battery, processes). |
| `"clipboard"` | `desktop.Clipboard` | Read and write the operating system clipboard. |
| `"notifications"` | `desktop.Notifications` | Raise desktop notification toasts. |
| `"audio"` | `desktop.Audio` | Play sound effects through the native audio backend. |
| `"rbxassetid"` | — | Allows loading `rbxassetid://` asset URIs. |

### 2. Host-Only Permissions (Bundled Internal Surfaces)
These capabilities are strictly reserved for Dew's coordinator (bundled in `dew.exe`, such as `dashboard` and `quickpanel`). If an external applet declares them, the engine refuses to run:

- `"auth"` — User authentication and token refresh.
- `"discover"` — Browsing marketplace recipes.
- `"install"` — Installing or updating applet archives.
- `"library"` — Global applet supervisor (`dew.Library.List()`, starting/stopping processes).

---

## What Happens When a Permission Is Missing?

If a guest script attempts to call a capability that was not granted in `dew.toml`:
```luau
local widget = assert(desktop.Widget, "dew.toml must grant `widget`")
```
`desktop.Widget` evaluates to `nil`, triggering an immediate, informative runtime assertion.

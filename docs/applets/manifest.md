---
title: Manifest
description: Every field and permission you can set in dew.toml.
sidebar:
  order: 3
---

Every applet has a `dew.toml` file in its directory. It names the applet and lists
what the applet can reach. Dew reads it before running any of your code, so a
problem in the manifest is reported before your applet starts.

```toml title="dew.toml"
id = "clock"
name = "Clock"
description = "The time, in a corner of the desktop"
permissions = ["widget"]
```

## Fields

| Field | Type | Required | What it does |
| :--- | :--- | :--- | :--- |
| `id` | string | Yes | Names the entry module: Dew loads `<id>.luau`, or `main.luau` if that does not exist. |
| `name` | string | No | The window title, unless the surface sets its own, and the tray tooltip |
| `description` | string | No | The tray tooltip |
| `permissions` | list of strings | No | What the applet can reach. Defaults to nothing. |
| `hotkeys` | table | No | Not active yet. Dew reports each binding as not bound. |
| `experimentalDatamodel` | list of strings | No | Experimental features the applet uses, each with its version |

An `id` cannot be empty, `.` or `..`, and cannot contain `/`, `\` or `:`.

If `dew.toml` contains a field Dew does not recognize, Dew prints a message and
loads the applet anyway.

## Permissions

An applet with no permissions can draw nothing and reach nothing, so most applets
list at least one surface. Each permission adds one member to the
[`desktop` table](desktop.md):

| Permission | Adds | Lets your applet |
| :--- | :--- | :--- |
| `widget` | `desktop.Widget` | Open a borderless panel on the desktop |
| `window` | `desktop.Window` | Open an ordinary window |
| `overlay` | `desktop.Overlay` | Draw over the whole desktop |
| `popover` | `desktop.popover` | Reserved for a future kind of surface |
| `storage` | `desktop.Storage` | Save text that persists between runs |
| `clipboard` | `desktop.Clipboard` | Read and write the clipboard |
| `notifications` | `desktop.Notifications` | Show desktop notifications |
| `audio` | `desktop.Audio` | Play sounds |
| `system` | `desktop.System` | Read CPU, memory, disk, network and battery usage |
| `rbxassetid` | Nothing on `desktop` | Load images from `rbxassetid://` addresses |

A permission Dew does not recognize stops the applet from loading, with a message
naming it.

> [!NOTE]
> Dew's own built-in applets also use the permissions `auth`, `discover`,
> `install` and `library`. Your applet cannot request these: an applet that lists
> one does not load.

## When a permission is missing

Something your manifest does not grant is `nil` on `desktop`. Calling it raises
an error on that line, which tells you which permission to add. To give a clearer
message, check first:

```luau
local widget = assert(desktop.Widget, "dew.toml must grant `widget`")
```

## Experimental features

Some DataModel features are available before their design is final. To use one,
list it as `Class.Property@version`, with the version you wrote your applet
against:

```toml
experimentalDatamodel = ["GuiObject.BlendingMode@1"]
```

The version is required. If the feature changes in a later version of Dew, your
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

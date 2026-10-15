---
title: desktop
description: Everything your applet can reach outside its own interface.
sidebar:
  label: desktop table
  order: 4
---

`desktop` is the table your applet uses to reach things outside its own
interface, such as the time, the clipboard or a surface to draw into. It is a
global while your entry module loads, and Dew passes the same table to `mount`.

Some members are always there, and the rest appear only when `dew.toml` grants
their permission. A member your manifest does not grant is `nil`, so code that
uses it fails with an error on that line.

For autocomplete in your editor, point `luau-lsp` at `types/desktop.d.luau` in
the Dew repository.

## Summary

| Member | Permission | What it is |
| :--- | :--- | :--- |
| `Time` | None | The wall clock |
| `Clock` | None | Frames and a monotonic clock |
| `Pointer` | None | Where the pointer is, and which buttons are down |
| `Input` | None | Input events as signals |
| `Text` | None | Text measurement |
| `Widget` | `widget` | Opens a widget |
| `Window` | `window` | Opens a window |
| `Overlay` | `overlay` | Opens an overlay |
| `popover` | `popover` | Reserved; no surface kind is behind it yet |
| `Storage` | `storage` | Key-value storage that persists between runs |
| `Clipboard` | `clipboard` | The system clipboard |
| `Notifications` | `notifications` | Desktop notifications |
| `Audio` | `audio` | Sounds |
| `System` | `system` | CPU, memory, disk, network, battery and processes |

## Time

`Time` reads the wall clock, so use it whenever your applet shows the time of
day.

| Member | Returns |
| :--- | :--- |
| `Time.now()` | Seconds since the Unix epoch, as a float |
| `Time.timezone_offset()` | The local offset from UTC, in seconds |
| `Time.local_time()` | A table with `sec`, `min`, `hour`, `day`, `month`, `year`, `wday` and `isdst` |

## Clock

`Clock` runs your code once per frame and gives you a clock to time animation
against.

| Member | Returns |
| :--- | :--- |
| `Clock.OnFrame(listener)` | Calls `listener(dt)` once per frame. Returns a function that unsubscribes. |
| `Clock.Now()` | Seconds since Dew's first frame. Never goes backward. |
| `Clock.Name` | The clock's name |

`Clock.Now()` counts from Dew's first frame rather than from the epoch, so it is
not the time of day. Use `Time` for that.

## Pointer

`Pointer` tells you where the pointer is and which mouse buttons are down at the
moment you ask.

| Member | Returns |
| :--- | :--- |
| `Pointer.Position()` | The pointer's position, or `nil` when it is not known |
| `Pointer.IsDown(button)` | Whether a mouse button is held. `button` defaults to `0`. |

## Input

`Input` delivers keys, mouse buttons, pointer movement and the wheel as signals,
the way Roblox delivers them through `UserInputService`. A press and its release
within one frame cannot be recovered by polling `Pointer`, so connect to these
instead.

| Member | Fires when |
| :--- | :--- |
| `Input.InputBegan` | A key or button goes down |
| `Input.InputEnded` | A key or button comes up |
| `Input.InputChanged` | The pointer moves or the wheel turns |

Connect a listener to each signal with `Connect`, which returns a connection.
Call its `Disconnect` method when you no longer want the events.

## Text

| Member | Returns |
| :--- | :--- |
| `Text.Measure(text, size, font)` | The width and height of `text` at `size`. `font` is optional. |

Dew answers immediately, and the size matches what Dew draws, so a box you size
from the answer fits its text. [Host services](../spec/host_services.md)
describes what you can rely on when you measure text.

## Surfaces

`Widget`, `Window` and `Overlay` each open a surface and return the root to build
your interface under. Each takes an optional table of options, and an option you
leave out takes the default shown below. The [Surfaces](surfaces.md) page
describes each kind of surface in full.

### Widget options

| Option | Type | Default |
| :--- | :--- | :--- |
| `width`, `height` | number | The applet's `size` |
| `anchor` | `"top-left"`, `"top-right"`, `"bottom-left"`, `"bottom-right"` or `"center"` | `"top-right"` |
| `offset` | `{ x, y }` | 24, 24 |
| `clickThrough` | boolean | `false` |
| `zOrder` | `"bottom"`, `"normal"` or `"topmost"` | `"topmost"` |
| `draggable` | boolean | `false` |
| `keepOnScreen` | boolean | `true` |
| `snapToEdges` | boolean | `false` |
| `savePosition` | boolean | `false` |

### Window options

| Option | Type | Default |
| :--- | :--- | :--- |
| `width`, `height` | number | The applet's `size` |
| `title` | string | The manifest's `name`, or its `id` |

### Overlay options

| Option | Type | Default |
| :--- | :--- | :--- |
| `width`, `height` | number | The size the overlay was designed for. In a window the screen supplies the size; a headless render uses this. |
| `zOrder` | `"bottom"`, `"normal"` or `"topmost"` | `"topmost"` |
| `clickThrough` | boolean | `false` |

## Storage

With the `storage` permission, `Storage` keeps string values under string keys,
and the values persist between runs.

| Member | Returns |
| :--- | :--- |
| `Storage.get(key)` | The stored string, or `nil` |
| `Storage.set(key, value)` | Nothing |

## Clipboard

With the `clipboard` permission, `Clipboard` reads and writes the text on the
clipboard.

| Member | Returns |
| :--- | :--- |
| `Clipboard.read()` | The clipboard's text |
| `Clipboard.write(text)` | Nothing |

## Notifications

With the `notifications` permission, `Notifications` shows the message you pass
as a desktop notification.

| Member | Returns |
| :--- | :--- |
| `Notifications.show(message)` | Nothing. Shows a desktop notification. |

## Audio

With the `audio` permission, `Audio` plays a sound that you choose by its name.

| Member | Returns |
| :--- | :--- |
| `Audio.chime(name)` | Nothing. Plays a named sound. |

## System

With the `system` permission, `System` reports what the computer is doing. Dew
takes a new sample once a second, so use `System.OnSample` to update your
interface when a new reading arrives.

| Member | Returns |
| :--- | :--- |
| `System.Cpu()` | CPU usage |
| `System.Memory()` | Memory usage |
| `System.Disks()` | A list of disks |
| `System.Network()` | Network throughput |
| `System.Processes(limit)` | The busiest processes, busiest first. `limit` defaults to 5, up to 16. |
| `System.Battery()` | The battery, or `nil` without one |
| `System.Info()` | A table with `os`, `uptime` and `cores` |
| `System.History(metric)` | Up to the last 60 samples of one metric, oldest first |
| `System.Sequence()` | How many samples Dew has taken |
| `System.OnSample(listener)` | Calls `listener(sequence)` after each new sample. Returns a function that unsubscribes. |

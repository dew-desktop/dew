---
title: Host services
description: Text measurement and frame timing, available everywhere applets run.
sidebar:
  order: 2
---

A **host** is a program that runs applets. Dew is a host, and so is any tool that
runs applets without opening a window, such as a test runner. Every host provides
two services that are not part of the DataModel: **text measurement** and **frame
timing**. This page describes what you can rely on from each, whichever host your
applet runs on.

In Dew, you reach both through the [`desktop` table](../applets/desktop.md):
`desktop.Text.Measure` and `desktop.Clock`. Another host can name them
differently.

The words MUST, MUST NOT and SHOULD in this page are requirements on a host, as
RFC 2119 defines them.

## Text measurement

Text measurement tells you how wide and tall a string is at a given text size,
before you draw it:

```text
measure(text, size, font?) -> width, height
```

A host MUST answer immediately, in the same call. Layout happens inside a frame,
and it cannot wait for an answer that arrives later.

- **Natural size.** The answer is the size the string takes up on its own, not
  how it wraps inside a box. To wrap text, measure against the box you already
  have.
- **Consistent answers.** The same string at the same size MUST measure the same
  within a frame, and the answer MUST match what the host draws.
- **Fonts are optional.** A host with only one font can ignore the `font`
  argument, but a host that accepts a font name MUST NOT measure a different font.

## Frame timing

A frame subscription calls your function once per frame, with the time in seconds
since the previous frame:

```text
onFrame(callback) -> unsubscribe
```

Subscribing MUST return a function that ends the subscription. Call it when your
interface goes away, so that nothing keeps running for an element that no longer
exists.

You can end a subscription from inside the callback, including another
listener's. A host MUST handle that safely.

A host also provides a clock for animation:

```text
now() -> seconds
```

- **Always moving forward.** The clock never goes backward, so subtracting two
  readings always gives the time between them. Its starting point can be
  anything.
- **Not a wall clock.** For the time of day, use the wall clock instead. In
  Dew, that is `desktop.Time`.

A host that runs its own frames MUST NOT let an applet advance a frame by hand,
because an applet that could do so could run every frame twice.

## Where to find them

| Host | Text measurement | Frame timing |
| :--- | :--- | :--- |
| Dew | `desktop.Text.Measure` | `desktop.Clock.OnFrame` and `desktop.Clock.Now` |
| Roblox | `TextService:GetTextSize` | `RunService.Heartbeat` |

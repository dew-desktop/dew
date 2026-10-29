---
title: Events and signals
description: Respond to changes and input with the events you know from Roblox.
sidebar:
  order: 2
---

Instances in Dew have the same tree and the same events as in Roblox. You find
instances by navigating the tree, and you respond to changes and input by
connecting functions to signals.

## The instance tree

Use the familiar methods to move around the tree:

- `GetChildren` and `GetDescendants`
- `FindFirstChild`, `FindFirstChildOfClass` and `FindFirstAncestor`
- `IsA`, `IsDescendantOf` and `Destroy`

## Signals

Connect a function to a signal with `Connect`. It returns a connection; call its
`Disconnect` to stop listening:

```luau
local connection = button.Activated:Connect(function()
	print("clicked")
end)

-- later
connection:Disconnect()
```

The signals include:

- **Changes:** `Changed`, and `GetPropertyChangedSignal` for one property
- **The tree:** `ChildAdded`, `ChildRemoved` and `Destroying`
- **The pointer:** `Activated`, `MouseEnter`, `MouseLeave` and the mouse button
  events
- **Input:** `InputBegan`, `InputChanged` and `InputEnded`

[Supported classes](classes.md#methods-and-events) lists every method and event
you can use.

## Differences from Roblox

Three things work differently:

- **Unchanged values.** Setting a property to the value it already has fires no
  event.
- **No `Wait`.** `signal:Wait()` raises an error. Connect a function instead.
- **Errors in handlers.** If a connected function errors, Dew reports the error,
  and the property change that fired it still takes effect.

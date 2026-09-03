# DataModel Standard: scope

<!-- GENERATED. Regenerate with:
       lune run scripts/fetch_api_surface.luau   # only to move the Roblox pin
       cargo run --manifest-path host/Cargo.toml --bin datamodel-surface -- --markdown > docs/datamodel_scope.md
     Do not edit by hand; edit the classification lists in the tool.
     The bin is datamodel-surface, hyphenated. This line said datamodel_surface and
     the command it gave had never run. -->

Measured against Roblox **0.728.0.7280895**, from the reflection database that ships
with `rbx_reflection_database`. It tracks Roblox releases, so re-running this
after an update is how the standard notices the platform moved.

**THE SUBJECT IS THE DEW HOST**, measured against the Roblox engine. Aether is a
headless framework that runs on top of a host, the way Ark UI runs on top of a
DOM; it is a consumer of this surface, never an implementation of it, and is not
required to conform. What must match is what a Luau application sees, **with or
without Aether**.

**136 of 138 in-scope properties accepted by the host.** 22 more are
excluded by decision, and 24 classes under `GuiObject` are in scope.

Of those 138, **35 are already honoured by Aether's
renderer**. That is not a conformance figure; it splits the backlog by cost.

## Coverage by class

The middle column is what AETHER'S RENDERER honours, not what the host accepts.
The two are different questions and the gap between them is the backlog: a
property the host stores but the pipeline ignores is stored and not drawn.

| Class | Renderable | In the class |
| :--- | ---: | ---: |
| `TextBox` | 20 | 64 |
| `TextButton` | 20 | 62 |
| `ImageButton` | 17 | 59 |
| `ScrollingFrame` | 15 | 56 |
| `TextLabel` | 20 | 56 |
| `ImageLabel` | 17 | 49 |
| `GuiButton` | 14 | 44 |
| `CanvasGroup` | 14 | 40 |
| `Frame` | 14 | 39 |
| `GuiLabel` | 14 | 38 |
| `GuiObject` | 14 | 38 |
| `UIPageLayout` | 4 | 18 |
| `UIStroke` | 6 | 15 |
| `UIListLayout` | 4 | 14 |
| `UIGradient` | 7 | 13 |
| `UIGridLayout` | 3 | 13 |
| `UITableLayout` | 4 | 13 |
| `UICorner` | 3 | 10 |
| `UIFlexItem` | 2 | 9 |
| `UIPadding` | 6 | 9 |

## Out of scope

Excluded by decision rather than by oversight. Each is a claim that a
conformant implementation may ignore it.

**Classes.** Video, viewports and chat windows are engine features rather than
layout: `VideoFrame`, `VideoDisplay`, `ViewportFrame`, `TextChannelWindow`, `RelativeGui`.

**Properties.** `Archivable`, `AutoLocalize`, `GamepadInputEnabled`, `HoverHapticEffect`, `NextSelectionDown`, `NextSelectionLeft`, `NextSelectionRight`, `NextSelectionUp`, `PressHapticEffect`, `RobloxLocked`, `RootLocalizationTable`, `Sandboxed`, `Selectable`, `SelectionBehaviorDown`, `SelectionBehaviorLeft`, `SelectionBehaviorRight`, `SelectionBehaviorUp`, `SelectionGroup`, `SelectionImageObject`, `SelectionOrder`, `ShowNativeInput`, `TouchInputEnabled`

## Names the host is split on

Accepted on one class and refused on another, because the same property name
is a different type on unrelated classes. `UIStroke.Color` is a `Color3` the
host takes; `UIGradient.Color` is a `ColorSequence` it does not.

These count as NOT accepted. Counting them the other way let one class mask
another, and reported the surface as 100% complete while two properties were
still unassignable.

- `Color`
- `Transparency`

## Property backlog

What conformance actually requires, split by what it costs. 2 properties.

### Host work only (2)

Aether's renderer already honours these, so the host has to accept, validate and
store them and nothing else has to change.

- `Color`, `Transparency`

### Host and rendering (0)


## Methods and events

The other half of what an application can reach, and the half that had never
been measured. `rbx_reflection_database` carries no methods and no events, so
this comes from Roblox's own API dump, pinned at **0.736.0.7361346** by
`scripts/fetch_api_surface.luau`.

**THE TWO IMPLEMENTATIONS ARE THE ROBLOX ENGINE AND THE DEW HOST.** Aether is a
headless framework that runs on top of a host, the way Ark UI runs on top of a
DOM. It is a consumer of this surface and never an implementation of it, so it
is not measured here and is not required to conform. What must match is what a
Luau application sees, **with or without Aether**.

**21 of 52 in-scope members implemented.**
32 more are excluded by decision, out of 84 reachable
(44 methods, 40 events).

Asked of `dew_host::datamodel::members::implements`, the predicate `__index`
consults before it hands a guest a function -- so nothing below is a claim this
document makes on the host's behalf.

### Implemented

- `Changed`, `ChildAdded`, `ChildRemoved`, `ClearAllChildren`, `DescendantAdded`, `DescendantRemoving`, `Destroy`, `Destroying`
- `FindFirstAncestor`, `FindFirstAncestorOfClass`, `FindFirstAncestorWhichIsA`, `FindFirstChild`, `FindFirstChildOfClass`, `FindFirstChildWhichIsA`, `FindFirstDescendant`, `GetChildren`
- `GetDescendants`, `GetPropertyChangedSignal`, `IsA`, `IsAncestorOf`, `IsDescendantOf`

**6 of them are events**, reachable through `RBXScriptSignal` and
`RBXScriptConnection`. Every one is something an instance says about ITSELF --
its properties, its children, its own destruction. None of them is input:
that needs hit testing, which is a different problem.

### API backlog

What parity actually requires. No Dew guest can reach any of these.

- `Activated`, `CaptureFocus`, `FocusLost`, `Focused`, `GetScrollVelocity`, `InputBegan`, `InputChanged`, `InputEnded`
- `IsFocused`, `JumpTo`, `JumpToIndex`, `MouseButton1Click`, `MouseButton1Down`, `MouseButton1Up`, `MouseButton2Click`, `MouseButton2Down`
- `MouseButton2Up`, `MouseEnter`, `MouseLeave`, `MouseMoved`, `MouseWheelBackward`, `MouseWheelForward`, `Next`, `PageEnter`
- `PageLeave`, `Previous`, `ReleaseFocus`, `ResetScrollVelocity`, `SecondaryActivated`, `Stopped`, `WaitForChild`

### This document measures two Roblox builds at once

| half | build | pinned by |
| :--- | :--- | :--- |
| properties | 0.728.0.7280895 | whatever `rbx_reflection_database` ships |
| methods and events | 0.736.0.7361346 | `scripts/fetch_api_surface.luau` |

Conformance against two builds at once is not a thing an implementation can
satisfy. Closing this means moving the property half onto the pinned dump too,
or pinning the crate to the build the dump names.

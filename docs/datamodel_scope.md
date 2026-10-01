# DataModel Standard: scope

<!-- GENERATED. Regenerate with:
       lune run scripts/fetch_api_surface.luau   # only to move the engine pin
       cargo run --manifest-path host/Cargo.toml --bin datamodel-surface -- --markdown > docs/datamodel_scope.md
     Do not edit by hand; edit the classification lists in the tool.
     The bin is datamodel-surface, hyphenated. This line said datamodel_surface and
     the command it gave had never run. -->

Measured against the engine **0.741.19.7411056**, from the engine's API dump pinned
at that build by `scripts/fetch_api_surface.luau`. Both the property half
and the method/event half come from that one source, which is also the build
every verified conformance case cites.

**THE SUBJECT IS THE DEW HOST**, measured against the engine. Aether is a
headless framework that runs on top of a host, the way Ark UI runs on top of a
DOM; it is a consumer of this surface, never an implementation of it, and is not
required to conform. What must match is what a Luau application sees, **with or
without Aether**.

**139 of 139 in-scope properties accepted by the host.** 22 more are
excluded by decision, and 25 classes under `GuiObject` are in scope.

Of those 139, **35 are already honoured by Aether's
renderer**. That is not a conformance figure; it splits the backlog by cost.

## Coverage by class

The middle column is what AETHER'S RENDERER honours, not what the host accepts.
The two are different questions and the gap between them is the backlog: a
property the host stores but the pipeline ignores is stored and not drawn.

| Class | Renderable | In the class |
| :--- | ---: | ---: |
| `TextBox` | 19 | 63 |
| `TextButton` | 19 | 61 |
| `ImageButton` | 16 | 58 |
| `ScrollingFrame` | 14 | 55 |
| `TextLabel` | 19 | 55 |
| `ImageLabel` | 16 | 48 |
| `InputActionLabel` | 20 | 47 |
| `GuiButton` | 13 | 43 |
| `CanvasGroup` | 13 | 39 |
| `Frame` | 13 | 38 |
| `GuiLabel` | 13 | 37 |
| `GuiObject` | 13 | 37 |
| `UIPageLayout` | 4 | 18 |
| `UIStroke` | 6 | 15 |
| `UIListLayout` | 4 | 14 |
| `UIGradient` | 7 | 13 |
| `UIGridLayout` | 3 | 13 |
| `UITableLayout` | 4 | 13 |
| `UICorner` | 3 | 10 |
| `UIFlexItem` | 2 | 9 |

## Out of scope

Excluded by decision rather than by oversight. Each is a claim that a
conformant implementation may ignore it.

**Classes.** Video, viewports and chat windows are engine features rather than
layout: `VideoFrame`, `VideoDisplay`, `ViewportFrame`, `TextChannelWindow`, `RelativeGui`.

**Properties.** `Archivable`, `AutoLocalize`, `Capabilities`, `GamepadInputEnabled`, `HoverHapticEffect`, `NextSelectionDown`, `NextSelectionLeft`, `NextSelectionRight`, `NextSelectionUp`, `PressHapticEffect`, `RootLocalizationTable`, `Sandboxed`, `Selectable`, `SelectionBehaviorDown`, `SelectionBehaviorLeft`, `SelectionBehaviorRight`, `SelectionBehaviorUp`, `SelectionGroup`, `SelectionImageObject`, `SelectionOrder`, `ShowNativeInput`, `TouchInputEnabled`

## Property backlog

What conformance actually requires, split by what it costs. 0 properties.

### Host work only (0)

Aether's renderer already honours these, so the host has to accept, validate and
store them and nothing else has to change.


### Host and rendering (0)


## Methods and events

The other half of what an application can reach, from the same pinned dump at
**0.741.19.7411056**.

**THE TWO IMPLEMENTATIONS ARE THE ROBLOX ENGINE AND THE DEW HOST.** Aether is a
headless framework that runs on top of a host, the way Ark UI runs on top of a
DOM. It is a consumer of this surface and never an implementation of it, so it
is not measured here and is not required to conform. What must match is what a
Luau application sees, **with or without Aether**.

**49 of 57 in-scope members implemented.**
24 more are excluded by decision, out of 81 reachable
(41 methods, 40 events).

Asked of `dew_host::datamodel::members::implements`, the predicate `__index`
consults before it hands a guest a function -- so nothing below is a claim this
document makes on the host's behalf.

### Implemented

- `Activated`, `CaptureFocus`, `Changed`, `ChildAdded`, `ChildRemoved`, `ClearAllChildren`, `DescendantAdded`, `DescendantRemoving`
- `Destroy`, `Destroying`, `FindFirstAncestor`, `FindFirstAncestorOfClass`, `FindFirstAncestorWhichIsA`, `FindFirstChild`, `FindFirstChildOfClass`, `FindFirstChildWhichIsA`
- `FindFirstDescendant`, `FocusLost`, `Focused`, `GetAttribute`, `GetAttributes`, `GetChildren`, `GetDescendants`, `GetPropertyChangedSignal`
- `GetScrollVelocity`, `InputBegan`, `InputChanged`, `InputEnded`, `IsA`, `IsAncestorOf`, `IsDescendantOf`, `IsFocused`
- `MouseButton1Click`, `MouseButton1Down`, `MouseButton1Up`, `MouseButton2Click`, `MouseButton2Down`, `MouseButton2Up`, `MouseEnter`, `MouseLeave`
- `MouseMoved`, `MouseWheelBackward`, `MouseWheelForward`, `ReleaseFocus`, `ResetScrollVelocity`, `SecondaryActivated`, `SelectionGained`, `SelectionLost`
- `SetAttribute`

**26 of them are events**, reachable through `RBXScriptSignal` and
`RBXScriptConnection`.

**16 of them are input**, which is what makes a mod written
without a framework CLICKABLE. The host resolves the tree's geometry once
and both the painter and the hit test read that one answer, so what
responds to a click is what is on screen. The rest are what an instance
says about ITSELF -- its properties, its children, its own destruction.

### API backlog

What parity actually requires. No Dew guest can reach any of these.

- `JumpTo`, `JumpToIndex`, `Next`, `PageEnter`, `PageLeave`, `Previous`, `Stopped`, `WaitForChild`

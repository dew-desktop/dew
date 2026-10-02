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

Counted per class rather than by name, those properties make 471 (class,
property) pairs. **246 of the 471 are rendered by Dew**, 19 of them
partially; see the two sections below.

Of the 139 names, **36 are honoured by Aether's pipeline**.
That is not a conformance figure; it splits the backlog by cost.

## Coverage by class

Three different questions, asked of every property in scope for each class:

- **Accepted by the host**: an assignment to it is stored. Asked of
  `dew_host::datamodel::accepts`.
- **Rendered by Dew**: Dew's renderer reads it on an instance of THIS class,
  so changing it changes what is drawn or where. Asked of
  `dew_host::datamodel::render::honours`, which answers per class. Partial
  answers are counted here and shown in their own column, with what is
  missing listed below. A property with no paint of its own counts as not
  rendered. A test holds every claim to a gallery variant that moved pixels,
  a passing engine-verified conformance case, or a stated excuse.
- **Honoured by Aether's pipeline**: named by Aether's `Layout.Inputs` or its
  display list, by NAME and not by class. It describes Aether, not Dew, and
  splits the backlog by cost.

| Class | In scope | Accepted by the host | Rendered by Dew | of which partial | Honoured by Aether's pipeline |
| :--- | ---: | ---: | ---: | ---: | ---: |
| `TextBox` | 45 | 45 | 22 | 2 | 20 |
| `TextButton` | 42 | 42 | 22 | 2 | 20 |
| `ImageButton` | 39 | 39 | 19 | 1 | 15 |
| `ScrollingFrame` | 38 | 38 | 14 | 0 | 13 |
| `TextLabel` | 38 | 38 | 22 | 2 | 20 |
| `ImageLabel` | 31 | 31 | 19 | 1 | 15 |
| `InputActionLabel` | 30 | 30 | 20 | 2 | 20 |
| `GuiButton` | 24 | 24 | 12 | 0 | 12 |
| `CanvasGroup` | 22 | 22 | 12 | 0 | 12 |
| `Frame` | 21 | 21 | 12 | 0 | 12 |
| `GuiLabel` | 20 | 20 | 12 | 0 | 12 |
| `GuiObject` | 20 | 20 | 12 | 0 | 12 |
| `UIPageLayout` | 13 | 13 | 0 | 0 | 3 |
| `UIStroke` | 12 | 12 | 4 | 3 | 5 |
| `UIListLayout` | 11 | 11 | 10 | 0 | 3 |
| `UIGradient` | 10 | 10 | 6 | 0 | 6 |
| `UIGridLayout` | 10 | 10 | 9 | 0 | 2 |
| `UITableLayout` | 10 | 10 | 0 | 0 | 3 |
| `UICorner` | 7 | 7 | 2 | 1 | 2 |
| `UIFlexItem` | 6 | 6 | 5 | 0 | 1 |
| `UIPadding` | 6 | 6 | 5 | 4 | 5 |
| `UIAspectRatioConstraint` | 5 | 5 | 4 | 1 | 1 |
| `UISizeConstraint` | 4 | 4 | 3 | 0 | 1 |
| `UITextSizeConstraint` | 4 | 4 | 0 | 0 | 1 |
| `UIScale` | 3 | 3 | 0 | 0 | 2 |

## Rendered by Dew, by class

What `render::honours` answers for each in-scope property, so every count in
the table above can be traced to the names behind it.

### `TextBox`

- **Rendered:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Text`, `TextColor3`, `TextScaled`, `TextSize`, `TextTransparency`, `TextWrapped`, `TextXAlignment`, `TextYAlignment`, `Visible`, `ZIndex`
- **Partial,** `Font`: a family Dew ships, or finds in a local Studio install, is drawn in its own face; any other is drawn in the nearest shipped face
- **Partial,** `FontFace`: a family Dew ships, or finds in a local Studio install, is drawn in its own face; any other is drawn in the nearest shipped face
- **Not rendered:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `ClearTextOnFocus`, `CursorPosition`, `InputSink`, `Interactable`, `LineHeight`, `MaxVisibleGraphemes`, `MultiLine`, `OpenTypeFeatures`, `PlaceholderColor3`, `PlaceholderText`, `RichText`, `Rotation`, `SelectionStart`, `SizeConstraint`, `TextDirection`, `TextEditable`, `TextStrokeColor3`, `TextStrokeTransparency`, `TextTruncate`

### `TextButton`

- **Rendered:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Text`, `TextColor3`, `TextScaled`, `TextSize`, `TextTransparency`, `TextWrapped`, `TextXAlignment`, `TextYAlignment`, `Visible`, `ZIndex`
- **Partial,** `Font`: a family Dew ships, or finds in a local Studio install, is drawn in its own face; any other is drawn in the nearest shipped face
- **Partial,** `FontFace`: a family Dew ships, or finds in a local Studio install, is drawn in its own face; any other is drawn in the nearest shipped face
- **Not rendered:** `Active`, `AutoButtonColor`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `LineHeight`, `MaxVisibleGraphemes`, `Modal`, `OpenTypeFeatures`, `RichText`, `Rotation`, `Selected`, `SizeConstraint`, `Style`, `TextDirection`, `TextStrokeColor3`, `TextStrokeTransparency`, `TextTruncate`

### `ImageButton`

- **Rendered:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `Image`, `ImageColor3`, `ImageContent`, `ImageRectOffset`, `ImageRectSize`, `ImageTransparency`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **Partial,** `ScaleType`: Stretch, Fit and Crop are drawn; Slice and Tile fall back to Stretch
- **Not rendered:** `Active`, `AutoButtonColor`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `HoverImage`, `HoverImageContent`, `InputSink`, `Interactable`, `Modal`, `PressedImage`, `PressedImageContent`, `ResampleMode`, `Rotation`, `Selected`, `SizeConstraint`, `SliceCenter`, `SliceScale`, `Style`, `TileSize`

### `ScrollingFrame`

- **Rendered:** `AnchorPoint`, `AutomaticCanvasSize`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `CanvasPosition`, `CanvasSize`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **Not rendered:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `BottomImage`, `BottomImageContent`, `ClipsDescendants`, `ElasticBehavior`, `HorizontalScrollBarInset`, `InputSink`, `Interactable`, `MidImage`, `MidImageContent`, `Rotation`, `ScrollBarImageColor3`, `ScrollBarImageTransparency`, `ScrollBarThickness`, `ScrollingDirection`, `ScrollingEnabled`, `SizeConstraint`, `TopImage`, `TopImageContent`, `VerticalScrollBarInset`, `VerticalScrollBarPosition`

### `TextLabel`

- **Rendered:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Text`, `TextColor3`, `TextScaled`, `TextSize`, `TextTransparency`, `TextWrapped`, `TextXAlignment`, `TextYAlignment`, `Visible`, `ZIndex`
- **Partial,** `Font`: a family Dew ships, or finds in a local Studio install, is drawn in its own face; any other is drawn in the nearest shipped face
- **Partial,** `FontFace`: a family Dew ships, or finds in a local Studio install, is drawn in its own face; any other is drawn in the nearest shipped face
- **Not rendered:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `LineHeight`, `MaxVisibleGraphemes`, `OpenTypeFeatures`, `RichText`, `Rotation`, `SizeConstraint`, `TextDirection`, `TextStrokeColor3`, `TextStrokeTransparency`, `TextTruncate`

### `ImageLabel`

- **Rendered:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `Image`, `ImageColor3`, `ImageContent`, `ImageRectOffset`, `ImageRectSize`, `ImageTransparency`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **Partial,** `ScaleType`: Stretch, Fit and Crop are drawn; Slice and Tile fall back to Stretch
- **Not rendered:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `ResampleMode`, `Rotation`, `SizeConstraint`, `SliceCenter`, `SliceScale`, `TileSize`

### `InputActionLabel`

- **Rendered:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `TextColor3`, `TextSize`, `TextTransparency`, `TextWrapped`, `TextXAlignment`, `TextYAlignment`, `Visible`, `ZIndex`
- **Partial,** `FontFace`: a family Dew ships, or finds in a local Studio install, is drawn in its own face; any other is drawn in the nearest shipped face
- **Partial,** `InputAction`: drawn as its own name in text; no glyph for the bound input
- **Not rendered:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `ImageColor3`, `ImageTransparency`, `InputSink`, `Interactable`, `Rotation`, `SizeConstraint`

### `GuiButton`

- **Rendered:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **Not rendered:** `Active`, `AutoButtonColor`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `Modal`, `Rotation`, `Selected`, `SizeConstraint`, `Style`

### `CanvasGroup`

- **Rendered:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **Not rendered:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `GroupColor3`, `GroupTransparency`, `InputSink`, `Interactable`, `Rotation`, `SizeConstraint`

### `Frame`

- **Rendered:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **Not rendered:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `Rotation`, `SizeConstraint`, `Style`

### `GuiLabel`

- **Rendered:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **Not rendered:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `Rotation`, `SizeConstraint`

### `GuiObject`

- **Rendered:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **Not rendered:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `Rotation`, `SizeConstraint`

### `UIPageLayout`

- **Not rendered:** `Animated`, `Circular`, `EasingDirection`, `EasingStyle`, `FillDirection`, `HorizontalAlignment`, `Name`, `Padding`, `Parent`, `ScrollWheelInputEnabled`, `SortOrder`, `TweenTime`, `VerticalAlignment`

### `UIStroke`

- **Rendered:** `Parent`
- **Partial,** `Color`: always drawn around the parent's box; on a text parent the engine's default outlines the glyphs
- **Partial,** `Thickness`: always drawn around the parent's box; on a text parent the engine's default outlines the glyphs
- **Partial,** `Transparency`: always drawn around the parent's box; on a text parent the engine's default outlines the glyphs
- **Not rendered:** `ApplyStrokeMode`, `BorderOffset`, `BorderStrokePosition`, `Enabled`, `LineJoinMode`, `Name`, `StrokeSizingMode`, `ZIndex`

### `UIListLayout`

- **Rendered:** `FillDirection`, `HorizontalAlignment`, `HorizontalFlex`, `ItemLineAlignment`, `Padding`, `Parent`, `SortOrder`, `VerticalAlignment`, `VerticalFlex`, `Wraps`
- **Not rendered:** `Name`

### `UIGradient`

- **Rendered:** `Color`, `Enabled`, `Parent`, `Rotation`, `Transparency`, `Type`
- **Not rendered:** `Name`, `Offset`, `Scale`, `TileMode`

### `UIGridLayout`

- **Rendered:** `CellPadding`, `CellSize`, `FillDirection`, `FillDirectionMaxCells`, `HorizontalAlignment`, `Parent`, `SortOrder`, `StartCorner`, `VerticalAlignment`
- **Not rendered:** `Name`

### `UITableLayout`

- **Not rendered:** `FillDirection`, `FillEmptySpaceColumns`, `FillEmptySpaceRows`, `HorizontalAlignment`, `MajorAxis`, `Name`, `Padding`, `Parent`, `SortOrder`, `VerticalAlignment`

### `UICorner`

- **Rendered:** `Parent`
- **Partial,** `CornerRadius`: the offset rounds the corners; the scale is dropped
- **Not rendered:** `BottomLeftRadius`, `BottomRightRadius`, `Name`, `TopLeftRadius`, `TopRightRadius`

### `UIFlexItem`

- **Rendered:** `FlexMode`, `GrowRatio`, `ItemLineAlignment`, `Parent`, `ShrinkRatio`
- **Not rendered:** `Name`

### `UIPadding`

- **Rendered:** `Parent`
- **Partial,** `PaddingBottom`: the offset insets the content; the scale is discarded
- **Partial,** `PaddingLeft`: the offset insets the content; the scale is discarded
- **Partial,** `PaddingRight`: the offset insets the content; the scale is discarded
- **Partial,** `PaddingTop`: the offset insets the content; the scale is discarded
- **Not rendered:** `Name`

### `UIAspectRatioConstraint`

- **Rendered:** `AspectRatio`, `AspectType`, `Parent`
- **Partial,** `DominantAxis`: read only under ScaleWithParentSize; FitWithinMaxSize ignores it, which is unverified
- **Not rendered:** `Name`

### `UISizeConstraint`

- **Rendered:** `MaxSize`, `MinSize`, `Parent`
- **Not rendered:** `Name`

### `UITextSizeConstraint`

- **Not rendered:** `MaxTextSize`, `MinTextSize`, `Name`, `Parent`

### `UIScale`

- **Not rendered:** `Name`, `Parent`, `Scale`

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

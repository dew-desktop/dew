---
title: Supported classes
description: The Roblox UI classes Dew supports, and how each one works.
sidebar:
  order: 1
---

<!-- GENERATED. Regenerate with:
       cargo run --manifest-path host/Cargo.toml --bin datamodel-surface -- --markdown
     Do not edit by hand; edit the classification lists in the tool. -->

Dew provides the user interface classes you know from Roblox, such as `Frame`,
`TextLabel` and `UIListLayout`. You create them with `Instance.new` and set their
properties the same way you would in Roblox.

For each class, this page lists the properties that take effect, the ones that
take effect in part, and the ones that behave differently from Roblox.

> [!NOTE]
> You can set and read every property on this page, including the ones that
> have no effect yet.

## CanvasGroup

- **Supported:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **No effect yet:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `GroupColor3`, `GroupTransparency`, `InputSink`, `Interactable`, `Rotation`, `SizeConstraint`

## Frame

- **Supported:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **No effect yet:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `Rotation`, `SizeConstraint`, `Style`

## GuiButton

- **Supported:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **No effect yet:** `Active`, `AutoButtonColor`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `Modal`, `Rotation`, `Selected`, `SizeConstraint`, `Style`

## GuiLabel

- **Supported:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **No effect yet:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `Rotation`, `SizeConstraint`

## GuiObject

- **Supported:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **No effect yet:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `Rotation`, `SizeConstraint`

## ImageButton

- **Supported:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `Image`, `ImageColor3`, `ImageContent`, `ImageRectOffset`, `ImageRectSize`, `ImageTransparency`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **Partly supported:** `ScaleType`: Stretch, Fit and Crop are drawn; Slice and Tile fall back to Stretch
- **No effect yet:** `Active`, `AutoButtonColor`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `HoverImage`, `HoverImageContent`, `InputSink`, `Interactable`, `Modal`, `PressedImage`, `PressedImageContent`, `ResampleMode`, `Rotation`, `Selected`, `SizeConstraint`, `SliceCenter`, `SliceScale`, `Style`, `TileSize`

## ImageLabel

- **Supported:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `Image`, `ImageColor3`, `ImageContent`, `ImageRectOffset`, `ImageRectSize`, `ImageTransparency`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **Partly supported:** `ScaleType`: Stretch, Fit and Crop are drawn; Slice and Tile fall back to Stretch
- **No effect yet:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `ResampleMode`, `Rotation`, `SizeConstraint`, `SliceCenter`, `SliceScale`, `TileSize`

## InputActionLabel

- **Supported:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `TextColor3`, `TextSize`, `TextTransparency`, `TextWrapped`, `TextXAlignment`, `TextYAlignment`, `Visible`, `ZIndex`
- **Partly supported:** `FontFace`: fonts Dew includes, or finds in a local Roblox Studio install, are drawn as themselves; any other font is drawn in the closest one Dew includes
- **Partly supported:** `InputAction`: shown as the action's name; no icon for the bound input is drawn
- **No effect yet:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `ImageColor3`, `ImageTransparency`, `InputSink`, `Interactable`, `Rotation`, `SizeConstraint`

## ScrollingFrame

- **Supported:** `AnchorPoint`, `AutomaticCanvasSize`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `CanvasPosition`, `CanvasSize`, `LayoutOrder`, `Name`, `Parent`, `Position`, `Size`, `Visible`, `ZIndex`
- **No effect yet:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `BottomImage`, `BottomImageContent`, `ClipsDescendants`, `ElasticBehavior`, `HorizontalScrollBarInset`, `InputSink`, `Interactable`, `MidImage`, `MidImageContent`, `Rotation`, `ScrollBarImageColor3`, `ScrollBarImageTransparency`, `ScrollBarThickness`, `ScrollingDirection`, `ScrollingEnabled`, `SizeConstraint`, `TopImage`, `TopImageContent`, `VerticalScrollBarInset`, `VerticalScrollBarPosition`

## TextBox

- **Supported:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `LineHeight`, `Name`, `Parent`, `PlaceholderColor3`, `Position`, `Size`, `Text`, `TextColor3`, `TextScaled`, `TextSize`, `TextStrokeColor3`, `TextStrokeTransparency`, `TextTransparency`, `TextWrapped`, `TextXAlignment`, `TextYAlignment`, `Visible`, `ZIndex`
- **Partly supported:** `CursorPosition`: shown as the caret and selection of a focused box; positions count UTF-8 bytes, so they can differ from Roblox in text beyond ASCII
- **Partly supported:** `Font`: fonts Dew includes, or finds in a local Roblox Studio install, are drawn as themselves; any other font is drawn in the closest one Dew includes
- **Partly supported:** `FontFace`: fonts Dew includes, or finds in a local Roblox Studio install, are drawn as themselves; any other font is drawn in the closest one Dew includes
- **Partly supported:** `RichText`: markup is removed from the text; bold, italic, color and other tags are not drawn
- **Partly supported:** `SelectionStart`: shown as the caret and selection of a focused box; positions count UTF-8 bytes, so they can differ from Roblox in text beyond ASCII
- **Partly supported:** `TextTruncate`: AtEnd shows an ellipsis, except while the box has focus, when the whole line scrolls; SplitWord behaves like None
- **Differs from Roblox:** `PlaceholderText`: rich text markup in a placeholder is applied; Roblox shows the markup as written
- **No effect yet:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `ClearTextOnFocus`, `InputSink`, `Interactable`, `MaxVisibleGraphemes`, `MultiLine`, `OpenTypeFeatures`, `Rotation`, `SizeConstraint`, `TextDirection`, `TextEditable`

## TextButton

- **Supported:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `LineHeight`, `Name`, `Parent`, `Position`, `Size`, `Text`, `TextColor3`, `TextScaled`, `TextSize`, `TextStrokeColor3`, `TextStrokeTransparency`, `TextTransparency`, `TextWrapped`, `TextXAlignment`, `TextYAlignment`, `Visible`, `ZIndex`
- **Partly supported:** `Font`: fonts Dew includes, or finds in a local Roblox Studio install, are drawn as themselves; any other font is drawn in the closest one Dew includes
- **Partly supported:** `FontFace`: fonts Dew includes, or finds in a local Roblox Studio install, are drawn as themselves; any other font is drawn in the closest one Dew includes
- **Partly supported:** `RichText`: markup is removed from the text; bold, italic, color and other tags are not drawn
- **Partly supported:** `TextTruncate`: AtEnd shows an ellipsis; SplitWord behaves like None
- **No effect yet:** `Active`, `AutoButtonColor`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `MaxVisibleGraphemes`, `Modal`, `OpenTypeFeatures`, `Rotation`, `Selected`, `SizeConstraint`, `Style`, `TextDirection`

## TextLabel

- **Supported:** `AnchorPoint`, `AutomaticSize`, `BackgroundColor3`, `BackgroundTransparency`, `ClipsDescendants`, `LayoutOrder`, `LineHeight`, `Name`, `Parent`, `Position`, `Size`, `Text`, `TextColor3`, `TextScaled`, `TextSize`, `TextStrokeColor3`, `TextStrokeTransparency`, `TextTransparency`, `TextWrapped`, `TextXAlignment`, `TextYAlignment`, `Visible`, `ZIndex`
- **Partly supported:** `Font`: fonts Dew includes, or finds in a local Roblox Studio install, are drawn as themselves; any other font is drawn in the closest one Dew includes
- **Partly supported:** `FontFace`: fonts Dew includes, or finds in a local Roblox Studio install, are drawn as themselves; any other font is drawn in the closest one Dew includes
- **Partly supported:** `RichText`: markup is removed from the text; bold, italic, color and other tags are not drawn
- **Partly supported:** `TextTruncate`: AtEnd shows an ellipsis; SplitWord behaves like None
- **No effect yet:** `Active`, `BorderColor3`, `BorderMode`, `BorderSizePixel`, `InputSink`, `Interactable`, `MaxVisibleGraphemes`, `OpenTypeFeatures`, `Rotation`, `SizeConstraint`, `TextDirection`

## UIAspectRatioConstraint

- **Supported:** `AspectRatio`, `AspectType`, `Parent`
- **Partly supported:** `DominantAxis`: used only with ScaleWithParentSize; FitWithinMaxSize ignores it
- **No effect yet:** `Name`

## UICorner

- **Supported:** `Parent`
- **Partly supported:** `CornerRadius`: the offset rounds the corners; the scale is ignored
- **No effect yet:** `BottomLeftRadius`, `BottomRightRadius`, `Name`, `TopLeftRadius`, `TopRightRadius`

## UIFlexItem

- **Supported:** `FlexMode`, `GrowRatio`, `ItemLineAlignment`, `Parent`, `ShrinkRatio`
- **No effect yet:** `Name`

## UIGradient

- **Supported:** `Color`, `Enabled`, `Parent`, `Rotation`, `Transparency`, `Type`
- **No effect yet:** `Name`, `Offset`, `Scale`, `TileMode`

## UIGridLayout

- **Supported:** `CellPadding`, `CellSize`, `FillDirection`, `FillDirectionMaxCells`, `HorizontalAlignment`, `Parent`, `SortOrder`, `StartCorner`, `VerticalAlignment`
- **No effect yet:** `Name`

## UIListLayout

- **Supported:** `FillDirection`, `HorizontalAlignment`, `HorizontalFlex`, `ItemLineAlignment`, `Padding`, `Parent`, `SortOrder`, `VerticalAlignment`, `VerticalFlex`, `Wraps`
- **No effect yet:** `Name`

## UIPadding

- **Supported:** `Parent`
- **Partly supported:** `PaddingBottom`: the offset is applied; the scale is ignored
- **Partly supported:** `PaddingLeft`: the offset is applied; the scale is ignored
- **Partly supported:** `PaddingRight`: the offset is applied; the scale is ignored
- **Partly supported:** `PaddingTop`: the offset is applied; the scale is ignored
- **No effect yet:** `Name`

## UIPageLayout

- **No effect yet:** `Animated`, `Circular`, `EasingDirection`, `EasingStyle`, `FillDirection`, `HorizontalAlignment`, `Name`, `Padding`, `Parent`, `ScrollWheelInputEnabled`, `SortOrder`, `TweenTime`, `VerticalAlignment`

## UIScale

- **No effect yet:** `Name`, `Parent`, `Scale`

## UISizeConstraint

- **Supported:** `MaxSize`, `MinSize`, `Parent`
- **No effect yet:** `Name`

## UIStroke

- **Supported:** `ApplyStrokeMode`, `BorderStrokePosition`, `Color`, `Parent`
- **Partly supported:** `Thickness`: around text, the outline always has round corners
- **Partly supported:** `Transparency`: around text, a translucent outline looks more solid than in Roblox
- **No effect yet:** `BorderOffset`, `Enabled`, `LineJoinMode`, `Name`, `StrokeSizingMode`, `ZIndex`

## UITableLayout

- **No effect yet:** `FillDirection`, `FillEmptySpaceColumns`, `FillEmptySpaceRows`, `HorizontalAlignment`, `MajorAxis`, `Name`, `Padding`, `Parent`, `SortOrder`, `VerticalAlignment`

## UITextSizeConstraint

- **No effect yet:** `MaxTextSize`, `MinTextSize`, `Name`, `Parent`

## Not available

These classes are not part of Dew: `VideoFrame`, `VideoDisplay`, `ViewportFrame`, `TextChannelWindow`, `RelativeGui`.

These properties are not part of Dew: `Archivable`, `AutoLocalize`, `Capabilities`, `GamepadInputEnabled`, `HoverHapticEffect`, `NextSelectionDown`, `NextSelectionLeft`, `NextSelectionRight`, `NextSelectionUp`, `PressHapticEffect`, `RootLocalizationTable`, `Sandboxed`, `Selectable`, `SelectionBehaviorDown`, `SelectionBehaviorLeft`, `SelectionBehaviorRight`, `SelectionBehaviorUp`, `SelectionGroup`, `SelectionImageObject`, `SelectionOrder`, `ShowNativeInput`, `TouchInputEnabled`.

## Methods and events

You can call these methods and connect to these events:

- `Activated`, `CaptureFocus`, `Changed`, `ChildAdded`, `ChildRemoved`, `ClearAllChildren`, `DescendantAdded`, `DescendantRemoving`
- `Destroy`, `Destroying`, `FindFirstAncestor`, `FindFirstAncestorOfClass`, `FindFirstAncestorWhichIsA`, `FindFirstChild`, `FindFirstChildOfClass`, `FindFirstChildWhichIsA`
- `FindFirstDescendant`, `FocusLost`, `Focused`, `GetAttribute`, `GetAttributes`, `GetChildren`, `GetDescendants`, `GetPropertyChangedSignal`
- `GetScrollVelocity`, `InputBegan`, `InputChanged`, `InputEnded`, `IsA`, `IsAncestorOf`, `IsDescendantOf`, `IsFocused`
- `MouseButton1Click`, `MouseButton1Down`, `MouseButton1Up`, `MouseButton2Click`, `MouseButton2Down`, `MouseButton2Up`, `MouseEnter`, `MouseLeave`
- `MouseMoved`, `MouseWheelBackward`, `MouseWheelForward`, `ReleaseFocus`, `ResetScrollVelocity`, `SecondaryActivated`, `SelectionGained`, `SelectionLost`
- `SetAttribute`

These are not available yet:

- `JumpTo`, `JumpToIndex`, `Next`, `PageEnter`, `PageLeave`, `Previous`, `Stopped`, `WaitForChild`

# DataModel Standard: scope

<!-- GENERATED. Regenerate with:
       cargo run --manifest-path host/Cargo.toml --bin datamodel_surface -- --markdown
     Do not edit by hand; edit the classification lists in the tool. -->

Measured against Roblox **0.728.0.7280895**, from the reflection database that ships
with `rbx_reflection_database`. It tracks Roblox releases, so re-running this
after an update is how the standard notices the platform moved.

**35 of 138 in-scope properties implemented.** 22 more are excluded by
decision, and 24 classes under `GuiObject` are in scope.

## Coverage by class

| Class | Implemented | In the class |
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

## Backlog

What conformance actually requires, and nothing else.

- `Active`, `Animated`, `ApplyStrokeMode`, `AspectRatio`, `AspectType`, `AutoButtonColor`, `AutomaticCanvasSize`, `BorderColor3`
- `BorderMode`, `BorderOffset`, `BorderSizePixel`, `BorderStrokePosition`, `BottomImage`, `BottomImageContent`, `BottomLeftRadius`, `BottomRightRadius`
- `CanvasSize`, `CellPadding`, `CellSize`, `Circular`, `ClearTextOnFocus`, `CursorPosition`, `DominantAxis`, `EasingDirection`
- `EasingStyle`, `ElasticBehavior`, `Enabled`, `FillDirectionMaxCells`, `FillEmptySpaceColumns`, `FillEmptySpaceRows`, `FlexMode`, `Font`
- `FontFace`, `GroupColor3`, `GroupTransparency`, `GrowRatio`, `HorizontalAlignment`, `HorizontalFlex`, `HorizontalScrollBarInset`, `HoverImage`
- `HoverImageContent`, `ImageContent`, `ImageRectOffset`, `ImageRectSize`, `InputSink`, `Interactable`, `ItemLineAlignment`, `LineHeight`
- `LineJoinMode`, `MajorAxis`, `MaxSize`, `MaxTextSize`, `MaxVisibleGraphemes`, `MidImage`, `MidImageContent`, `MinSize`
- `MinTextSize`, `Modal`, `MultiLine`, `OpenTypeFeatures`, `PlaceholderColor3`, `PlaceholderText`, `PressedImage`, `PressedImageContent`
- `ResampleMode`, `RichText`, `ScaleType`, `ScrollBarImageColor3`, `ScrollBarImageTransparency`, `ScrollBarThickness`, `ScrollWheelInputEnabled`, `ScrollingDirection`
- `ScrollingEnabled`, `Selected`, `SelectionStart`, `ShrinkRatio`, `SizeConstraint`, `SliceCenter`, `SliceScale`, `SortOrder`
- `StartCorner`, `StrokeSizingMode`, `Style`, `TextDirection`, `TextEditable`, `TextScaled`, `TextStrokeColor3`, `TextStrokeTransparency`
- `TextTruncate`, `TextWrapped`, `TileMode`, `TileSize`, `TopImage`, `TopImageContent`, `TopLeftRadius`, `TopRightRadius`
- `TweenTime`, `Type`, `VerticalAlignment`, `VerticalFlex`, `VerticalScrollBarInset`, `VerticalScrollBarPosition`, `Wraps`

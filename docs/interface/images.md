---
title: Images
description: Show images from your app's own files.
sidebar:
  order: 3
---

An `ImageLabel` or `ImageButton` shows an image. Put the image file in your app's
directory and name it with `mod://` followed by its path:

```luau
local icon = Instance.new("ImageLabel")
icon.Size = UDim2.new(0, 44, 0, 44)
icon.BackgroundTransparency = 1
icon.ImageContent = Content.fromUri("mod://droplet.png")
icon.ScaleType = Enum.ScaleType.Fit
icon.Parent = root
```

`icon.Image = "mod://droplet.png"` works too. If you set both, `ImageContent` wins.
A path that leads outside your app's directory is refused.

## Image properties

| Property | Behavior |
| :--- | :--- |
| `Image`, `ImageContent` | The image to draw |
| `ImageColor3` | Tints the image |
| `ImageTransparency` | Fades the image |
| `ImageRectOffset`, `ImageRectSize` | Draws part of the image, for sprite sheets |
| `ScaleType` | `Stretch`, `Fit` and `Crop` |

`Slice` and `Tile` scaling are not drawn yet, and neither are `SliceCenter`,
`SliceScale` and `TileSize`. Images that ask for them are stretched, and Dew
prints a message the first time.

## Missing images

Dew loads `mod://` images by default. An `rbxassetid://` image needs the
`rbxassetid` permission; without it, Dew prints the address once. An image Dew
cannot load draws as an empty, marked box, which is easier to spot than empty
space.

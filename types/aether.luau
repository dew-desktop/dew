--!strict
-- @dew/aether: Pure Luau Roblox-Mirrored UI Construction Layer & Virtual Compiler

local Aether = {}

-- 1. Roblox Core Data Types
export type UDim = { Scale: number, Offset: number }
export type UDim2 = { X: UDim, Y: UDim }
export type Vector2 = { X: number, Y: number }
export type Color3 = { R: number, G: number, B: number }

local function UDim_new(scale: number, offset: number): UDim
    return { Scale = scale, Offset = offset }
end

local function UDim2_new(scaleX: number, offsetX: number, scaleY: number, offsetY: number): UDim2
    return {
        X = { Scale = scaleX, Offset = offsetX },
        Y = { Scale = scaleY, Offset = offsetY }
    }
end

local function Vector2_new(x: number, y: number): Vector2
    return { X = x, Y = y }
end

local function Color3_fromRGB(r: number, g: number, b: number): Color3
    return { R = r / 255, G = g / 255, B = b / 255 }
end

local function Color3_fromHex(hex: string): Color3
    local clean = string.gsub(hex, "#", "")
    local num = tonumber(clean, 16) or 0
    local r = math.floor(num / 65536) % 256
    local g = math.floor(num / 256) % 256
    local b = num % 256
    return Color3_fromRGB(r, g, b)
end

Aether.UDim = { new = UDim_new }
Aether.UDim2 = { new = UDim2_new }
Aether.Vector2 = { new = Vector2_new }
Aether.Color3 = { fromRGB = Color3_fromRGB, fromHex = Color3_fromHex }

Aether.Enum = {
    Font = {
        Default = 0,
        Sans = 0,
        Code = 1,
        Mono = 1
    }
}

-- 2. Virtual Instance Constructor (New "Frame" { ... })
export type VirtualInstance = {
    ClassName: string,
    Props: { [string]: any },
    Children: { VirtualInstance }
}

function Aether.New(className: string)
    return function(props: { [any]: any }): VirtualInstance
        local cleanProps: { [string]: any } = {}
        local children: { VirtualInstance } = {}

        for k, v in pairs(props) do
            if type(k) == "number" and type(v) == "table" and v.ClassName then
                table.insert(children, v)
            elseif type(k) == "string" then
                cleanProps[k] = v
            end
        end

        return {
            ClassName = className,
            Props = cleanProps,
            Children = children
        }
    end
end

-- 3. Virtual Tree Compiler (Resolves Roblox UI Tree into GPU Draw Commands)
export type CompiledDrawCommand = {
    kind: string,
    x: number,
    y: number,
    w: number,
    h: number,
    radius: number,
    stroke_width: number,
    r: number,
    g: number,
    b: number,
    a: number,
    text: string?
}

function Aether.compileTree(root: VirtualInstance, parentBounds: { x: number, y: number, w: number, h: number }): { commands: { CompiledDrawCommand } }
    local commands: { CompiledDrawCommand } = {}

    local function traverse(node: VirtualInstance, parentX: number, parentY: number, parentW: number, parentH: number)
        local props = node.Props

        -- Resolve Size
        local size: UDim2 = props.Size or UDim2_new(0, parentW, 0, parentH)
        local w = (parentW * size.X.Scale) + size.X.Offset
        local h = (parentH * size.Y.Scale) + size.Y.Offset

        -- Resolve Position
        local pos: UDim2 = props.Position or UDim2_new(0, 0, 0, 0)
        local x = parentX + (parentW * pos.X.Scale) + pos.X.Offset
        local y = parentY + (parentH * pos.Y.Scale) + pos.Y.Offset

        -- Look for UICorner / UIStroke in children
        local cornerRadius = 0
        local strokeThickness = 0
        local strokeColor = nil
        local strokeAlpha = 255

        for _, child in ipairs(node.Children) do
            if child.ClassName == "UICorner" then
                local radiusProp: UDim = child.Props.CornerRadius or UDim_new(0, 0)
                cornerRadius = (math.min(w, h) * radiusProp.Scale) + radiusProp.Offset
            elseif child.ClassName == "UIStroke" then
                strokeThickness = child.Props.Thickness or 1
                strokeColor = child.Props.Color or Color3_fromRGB(255, 255, 255)
                local trans = child.Props.Transparency or 0
                strokeAlpha = math.floor((1.0 - trans) * 255)
            end
        end

        -- Render Background
        local bgTrans = props.BackgroundTransparency or 0
        if bgTrans < 1.0 and (node.ClassName == "Frame" or node.ClassName == "TextButton") then
            local bgColor: Color3 = props.BackgroundColor3 or Color3_fromRGB(255, 255, 255)
            local bgAlpha = math.floor((1.0 - bgTrans) * 255)

            table.insert(commands, {
                kind = "rounded_rect",
                x = x,
                y = y,
                w = w,
                h = h,
                radius = cornerRadius,
                stroke_width = 0,
                r = math.floor(bgColor.R * 255),
                g = math.floor(bgColor.G * 255),
                b = math.floor(bgColor.B * 255),
                a = bgAlpha,
                text = nil
            })
        end

        -- Render Stroke
        if strokeThickness > 0 and strokeColor then
            table.insert(commands, {
                kind = "stroke",
                x = x,
                y = y,
                w = w,
                h = h,
                radius = cornerRadius,
                stroke_width = strokeThickness,
                r = math.floor(strokeColor.R * 255),
                g = math.floor(strokeColor.G * 255),
                b = math.floor(strokeColor.B * 255),
                a = strokeAlpha,
                text = nil
            })
        end

        -- Render Text
        if (node.ClassName == "TextLabel" or node.ClassName == "TextButton") and props.Text then
            local textColor: Color3 = props.TextColor3 or Color3_fromRGB(255, 255, 255)
            local textSize: number = props.TextSize or 14
            local isMono = (props.Font == Aether.Enum.Font.Code or props.Font == Aether.Enum.Font.Mono) and 1.0 or 0.0

            table.insert(commands, {
                kind = "text",
                x = x,
                y = y,
                w = w,
                h = h,
                radius = isMono,
                stroke_width = textSize,
                r = math.floor(textColor.R * 255),
                g = math.floor(textColor.G * 255),
                b = math.floor(textColor.B * 255),
                a = 255,
                text = tostring(props.Text)
            })
        end

        -- Recursively Traverse Children
        for _, child in ipairs(node.Children) do
            if child.ClassName ~= "UICorner" and child.ClassName ~= "UIStroke" then
                traverse(child, x, y, w, h)
            end
        end
    end

    traverse(root, parentBounds.x, parentBounds.y, parentBounds.w, parentBounds.h)

    return { commands = commands }
end

return Aether

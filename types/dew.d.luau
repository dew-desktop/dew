--!strict
-- Dew Modding SDK Type Definitions (@dew/core)

export type WidgetRenderOutput = {
    text: string,
    subtext: string?,
    icon: string?,
    color: string?,
    glow: boolean?,
    badge: string?,
    progress: number? -- 0.0 to 1.0
}

export type WidgetDef = {
    id: string,
    title: string,
    priority: number?,
    render: () => WidgetRenderOutput,
    onClick: (() => ())?,
    onSecondaryClick: (() => ())?
}

export type PaletteOption = {
    id: string,
    title: string,
    subtitle: string?,
    icon: string?,
    action: () => ()
}

export type PaletteOptions = {
    placeholder: string?,
    options: {PaletteOption}?,
    onSubmit: ((query: string) -> ())?
}

export type NotificationOptions = {
    title: string?,
    body: string,
    icon: string?,
    sound: boolean?
}

export type DewCore = {
    hud: {
        registerWidget: (def: WidgetDef) -> (),
        unregisterWidget: (id: string) -> (),
        refresh: () -> (),
        expand: () -> (),
        collapse: () -> ()
    },
    hotkeys: {
        register: (hotkey: string, callback: () -> ()) -> (),
        unregister: (hotkey: string) -> ()
    },
    commandPalette: {
        open: (opts: PaletteOptions) -> (),
        close: () -> ()
    },
    notifications: {
        show: (textOrOpts: string | NotificationOptions) -> ()
    },
    storage: {
        get: (key: string) -> any?,
        set: (key: string, value: any) -> (),
        delete: (key: string) -> ()
    },
    clipboard: {
        readText: () -> string?,
        writeText: (text: string) -> ()
    },
    audio: {
        playChime: (chimeType: "complete" | "tick" | "alert" | "subtle") -> ()
    },
    time: {
        now: () -> number, -- Unix timestamp in seconds
        formatDuration: (seconds: number) -> string -- "01:23:45"
    }
}

declare local dew: DewCore
return dew

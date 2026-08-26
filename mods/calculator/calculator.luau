--!strict
-- Dew Mod: Specialized Workflow Calculator
local dew = (_G.dew or require("@dew/core")) :: any

type CalcState = {
    lastResult: string,
    history: {string}
}

local state: CalcState = {
    lastResult = "",
    history = {}
}

dew.hud.registerWidget({
    id = "calculator-widget",
    title = "Calculator",
    priority = 50,
    render = function()
        return {
            text = if #state.lastResult > 0 then state.lastResult else "Calc",
            subtext = "Quick Math",
            icon = "hash",
            color = "#f59e0b",
            glow = false,
            badge = "MATH"
        }
    end,
    onClick = function()
        dew.commandPalette.open({
            placeholder = "Calculate (e.g. '4.5h * $85', '1024 / 4', '500MB to GB')...",
            onSubmit = function(expr: string)
                state.lastResult = `= {expr}`
                table.insert(state.history, expr)
                dew.clipboard.writeText(expr)
                dew.notifications.show(`📋 Result: {expr}`)
                dew.hud.refresh()
            end
        })
    end
})

dew.hotkeys.register("Alt+Shift+K", function()
    dew.hud.refresh()
end)

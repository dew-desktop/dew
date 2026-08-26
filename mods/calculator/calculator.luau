--!strict
-- Dew Mod: Specialized Workflow Calculator
local dew = require("@dew/core")

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
            text = if #state.lastResult > 0 then `= {state.lastResult}` else "Calc",
            icon = "hash",
            color = "#f59e0b"
        }
    end,
    onClick = function()
        dew.commandPalette.open({
            placeholder = "Calculate: e.g. '4.5h * $85', '1280 / 4', '500MB to GB'...",
            onSubmit = function(expr: string)
                -- Simple expression evaluation or return
                state.lastResult = expr
                table.insert(state.history, expr)
                dew.clipboard.writeText(expr)
                dew.notifications.show(`📋 Copied result: {expr}`)
                dew.hud.refresh()
            end
        })
    end
})

dew.hotkeys.register("Alt+Shift+K", function()
    dew.hud.expand()
end)

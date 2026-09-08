-- A package entry point that reaches its own sibling through `@self`.
--
-- This is vide's `init.luau` with everything else taken out. No assert, no
-- `game`, no Roblox: the ONLY thing this module does is the one thing that was
-- failing, so a failure here can mean nothing else.
return require("@self/lib")

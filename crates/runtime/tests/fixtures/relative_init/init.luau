-- `./sibling` from an `init.luau` reaches a sibling of THE DIRECTORY, not a file
-- inside it. That is Luau's model and it is the reason `@self` exists: an
-- `init.luau` stands for its directory, so `./` is already one level out and a
-- package needs some other spelling to reach its own files.
return require("./sibling")

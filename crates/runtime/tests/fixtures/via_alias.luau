-- The same package reached the way a mod reaches one: through a host alias
-- rather than as the entry point. `@self` inside it must resolve identically,
-- and the two routes are tested separately because they arrive at the requirer
-- with different chunk names.
return require("@pkg")

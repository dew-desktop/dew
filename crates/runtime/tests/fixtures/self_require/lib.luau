-- The sibling `@self/lib` must reach. The marker is checked by the test, so a
-- resolver that found some OTHER `lib.luau` would not pass.
return { marker = "self-require-fixture" }

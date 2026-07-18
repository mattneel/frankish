-- A non-final explist prefix live across a suspending FINAL call:
-- the guard-before-consumption law (pack_with_tail never reads a
-- suspended dummy).
local function two()
  return coroutine.yield("in-two")
end
local co = coroutine.create(function()
  local a, b, c = "front", two()
  return a, b, c
end)
print(coroutine.resume(co))
print(coroutine.resume(co, "back1", "back2"))

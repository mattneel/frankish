-- wrap: strip-true delivery, args -> yield -> resume flow, final
-- return (the error re-raise paths are the fenced abort, unit-only).
local gen = coroutine.wrap(function(a, b)
  local x = coroutine.yield(a + b)
  coroutine.yield(x * 2)
  return "fin"
end)
print(gen(40, 2))
print(gen(5))
print(gen())

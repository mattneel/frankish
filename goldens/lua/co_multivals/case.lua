local co = coroutine.create(function(...)
  print("args", ...)
  local a, b, c = coroutine.yield(1, 2, 3)   -- yield multiple
  print("got", a, b, c)
  return "r1", "r2"
end)
print(coroutine.resume(co, "x", "y"))         -- resume passes to params
print(coroutine.resume(co, 10, 20, 30))       -- resume passes to yield's results

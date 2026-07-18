local inner = coroutine.create(function()
  coroutine.yield("inner-1")
  coroutine.yield("inner-2")
  return "inner-done"
end)
local outer = coroutine.create(function()
  print("outer: resume inner ->", coroutine.resume(inner))
  coroutine.yield("outer-1")                  -- yields OUTER only
  print("outer: resume inner ->", coroutine.resume(inner))
  print("outer: status(inner)", coroutine.status(inner))
  return "outer-done"
end)
print("main:", coroutine.resume(outer))
print("main: status inner/outer", coroutine.status(inner), coroutine.status(outer))
print("main:", coroutine.resume(outer))

-- Resuming your own resumer: the THIRD tuple message.
local outer = false
local inner = coroutine.create(function()
  print("inner:", coroutine.resume(outer))
  coroutine.yield("done-inner")
end)
outer = coroutine.create(function()
  print("outer: starting inner")
  print("outer:", coroutine.resume(inner))
  return "outer-done"
end)
print("main:", coroutine.resume(outer))

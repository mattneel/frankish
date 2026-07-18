local who = false
who = coroutine.create(function()
  print("self-status", coroutine.status(who))       -- running
end)
print(coroutine.resume(who))

local outer = false
local inner = coroutine.create(function()
  print("inner sees outer:", coroutine.status(outer)) -- normal
  coroutine.yield()
end)
outer = coroutine.create(function()
  coroutine.resume(inner)
  coroutine.yield("done-part")
end)
print(coroutine.resume(outer))

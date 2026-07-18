-- Body TAIL-CALLS yield: empty chain at the boundary; the next
-- resume's pack becomes the RETURN pack (multi-value). And a helper
-- tail-yield: the tail frame is absent (D-064 correctly), the resume
-- value flows to the CALLER's pending arithmetic.
local co = coroutine.create(function()
  return coroutine.yield("first")
end)
print(coroutine.resume(co))
print(coroutine.resume(co, "a", "b"))
print(coroutine.status(co))
local function tailhelper()
  return coroutine.yield("th")
end
local co2 = coroutine.create(function()
  local v = tailhelper()
  return v + 7
end)
print(coroutine.resume(co2))
print(coroutine.resume(co2, 100))

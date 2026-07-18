-- 800 frames of non-tail recursion suspend and resume (under the
-- interp's 1024 non-tail cap; far under lua5.1's own overflow).
local function descend(n)
  if n == 0 then
    return coroutine.yield("leaf") + 0
  end
  local r = descend(n - 1)
  return r + 1
end
local co = coroutine.create(function()
  return descend(800)
end)
print(coroutine.resume(co))
print(coroutine.resume(co, 5))

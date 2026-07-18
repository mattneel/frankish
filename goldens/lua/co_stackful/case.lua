-- yield ACROSS a plain function call: the helper is an ordinary
-- function, not a coroutine body — stackful suspension.
local function helper(n)
  for i = 1, n do
    coroutine.yield("helper", i)   -- yields THROUGH helper's frame
  end
  return "helper-done"
end
local co = coroutine.create(function()
  print("body: before helper")
  local r = helper(2)
  print("body: helper returned", r)
  coroutine.yield("body", 99)
  return "body-done"
end)
print(coroutine.resume(co))
print(coroutine.resume(co))
print(coroutine.resume(co))
print(coroutine.resume(co))
print(coroutine.status(co))

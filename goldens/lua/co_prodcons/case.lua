-- Producer/consumer over ipairs with yields in the loop body: the
-- GenFor triple survives body-position suspensions.
local items = {10, 20, 30}
local producer = coroutine.create(function()
  for i, v in ipairs(items) do
    coroutine.yield(i, v)
  end
  return "done"
end)
print(coroutine.resume(producer))
print(coroutine.resume(producer))
print(coroutine.resume(producer))
print(coroutine.resume(producer))

-- type(co) == "thread"; in-body SELF-RESUME answers the running
-- tuple (the status-before-walk law's standing witness); wrap as a
-- generic-for iterator (runs on the resumer's stack); resume-arg
-- explist adjustment.
local co = false
co = coroutine.create(function()
  print(type(co))
  print("self:", coroutine.resume(co))
  coroutine.yield("y")
end)
print(coroutine.resume(co))
local nums = coroutine.wrap(function()
  for i = 1, 3 do
    coroutine.yield(i, i * i)
  end
end)
for a, b in nums do
  print("for", a, b)
end
local co2 = coroutine.create(function(x)
  print("resume passed, first only:", x)
end)
print(coroutine.resume(co2, "one", "two"))

-- varargs: named params bind first, the tail is `...`
local function tally(label, ...)
  local t = { ... }
  local sum = 0
  for i, v in ipairs(t) do
    sum = sum + v
  end
  print(label)
  print(#t)
  print(sum)
end
tally("three", 10, 20, 30)
tally("none")

-- mid-explist truncation: `...` not in final position yields ONE value
local function first_of(...)
  local a, b = ..., 99
  print(a)
  print(b)
end
first_of(7, 8, 9)

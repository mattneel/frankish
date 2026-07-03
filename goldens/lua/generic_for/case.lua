-- pairs order is implementation-defined (canon rule): aggregate only.
local t = { alpha = 3, beta = 5, gamma = 7 }
local sum = 0
local names = 0
for k, v in pairs(t) do
  sum = sum + v
  names = names + #k
end
print(sum)
print(names)

-- ipairs is ordered: safe to print in sequence.
local list = { 10, 20, 30, 40 }
for i, v in ipairs(list) do
  print(i * 1000 + v)
end

-- next directly.
local single = { only = 42 }
local k2, v2 = next(single, nil)
print(k2)
print(v2)
print(next(single, k2))

local i = 0
repeat
  i = i + 1
until i >= 5
print(i)

local total = 0
local k = 0
while true do
  k = k + 1
  if k > 10 then
    break
  end
  total = total + k
end
print(total)

for j = 1, 100 do
  if j * j > 50 then
    print(j)
    break
  end
end

-- repeat's until sees body locals (Lua scoping).
local count = 0
repeat
  local double = count * 2
  count = count + 1
until double >= 6
print(count)

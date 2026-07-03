local n = 7
if n < 5 then
  print("small")
elseif n < 10 then
  print("medium")
else
  print("large")
end
local i = 0
local total = 0
while i < 100 do
  i = i + 1
  total = total + i
end
print(total)
for k = 1, 5 do
  print(k * k)
end
for k = 10, 1, -3 do
  print(k)
end

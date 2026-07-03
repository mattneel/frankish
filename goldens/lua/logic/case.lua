print(true and "yes" or "no")
print(false and "yes" or "no")
print(nil and 1)
print(nil or "fallback")
print(not nil)
print(not 0)
if 0 then
  print("zero is truthy")
end
local x = false
print(x or 42)

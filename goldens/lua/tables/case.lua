local t = { 10, 20, 30, name = "frankish", [7] = 99 }
print(t[1])
print(t[2] + t[3])
print(t.name)
print(t[7])
print(#t)
t[4] = 40
print(#t)
t[2] = nil
print(t[2])
local alias = t
alias[1] = 111
print(t[1])
print(t == alias)
print(t == { 10, 20, 30 })

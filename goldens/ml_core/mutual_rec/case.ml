let rec even n = if n = 0 then true else odd (n - 1)
and odd n = if n = 0 then false else even (n - 1)
let main () = if even 10 && odd 7 then 1 else 0

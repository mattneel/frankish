let classify n = match n with 0 -> 10 | 1 -> 20 | _ -> 30
let main () = classify 0 + classify 1 + classify 5 - 18

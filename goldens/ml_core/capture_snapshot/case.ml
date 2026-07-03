(* By-value capture: rebinding n later must not change the closure. *)
let main () =
  let n = 40 in
  let addn = fun x -> x + n in
  let n = 0 in
  addn 2 + n

(* A local recursive closure (the re-make pattern under the hood). *)
let main () =
  let rec sum n = if n = 0 then 0 else n + sum (n - 1) in
  sum 6 + 21

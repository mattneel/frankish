(* Sums and tuples crossing closure boundaries. *)
type opt = None | Some of int
let unwrap o = match o with None -> 0 | Some x -> x
let swap p = let (a, b) = p in (b, a)
let main () =
  let o = Some 40 in
  let f = fun d -> unwrap o + d in
  let (x, y) = swap (1, 1) in
  f 2 + x - y

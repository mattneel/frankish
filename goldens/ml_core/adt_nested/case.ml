type opt = None | Some of int
type box = Box of opt
let peek b = match b with
  | Box (Some x) -> x
  | Box None -> 0
let main () = peek (Box (Some 40)) + peek (Box None) + 2

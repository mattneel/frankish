type opt = None | Some of int
let get_or d o = match o with None -> d | Some x -> x
let main () = get_or 0 (Some 40) + get_or 2 None

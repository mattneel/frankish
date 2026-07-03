type shape = Circle of int | Rect of int * int | Point
let area s = match s with
  | Circle r -> 3 * r * r
  | Rect (w, h) -> w * h
  | Point -> 0
let main () = area (Rect (6, 7)) + area Point

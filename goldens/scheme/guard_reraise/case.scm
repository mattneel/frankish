(import (scheme base) (scheme write))
(display
  (guard (outer (#t (list 'outer-caught outer)))
    (guard (inner ((eq? inner 'nope) 'no))
      (raise 'boom))))
(newline)

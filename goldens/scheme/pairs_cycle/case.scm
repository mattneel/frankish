; A CYCLE of cons cells (D-077): set-cdr! ties the ring; walking by
; counter never terminates the structure, only the walk.
(import (scheme base) (scheme write))
(define (walk p n)
  (if (= n 0)
      (car p)
      (walk (cdr p) (- n 1))))
(let ((ring (list 7 11 13)))
  (set-cdr! (cdr (cdr ring)) ring)
  (display (walk ring 0)) (newline)
  (display (walk ring 4)) (newline)
  (display (walk ring 11)) (newline))

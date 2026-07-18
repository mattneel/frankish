; Pair mutation (D-077): set-car!/set-cdr! on SHARED structure —
; aliases observe writes (the boxed-cell representation made real).
(import (scheme base) (scheme write))
(let* ((p (cons 1 2))
       (alias p))
  (set-car! p 10)
  (set-cdr! p 20)
  (display (car alias)) (newline)
  (display (cdr alias)) (newline))
(let ((lst (list 1 2 3)))
  (set-car! (cdr lst) 99)
  (display (car (cdr lst))) (newline)
  (display (car lst)) (newline))

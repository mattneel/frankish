(import (scheme base) (scheme write))
; Escape out of a parameterize binding TWO params: both must restore.
(define p (make-parameter 1))
(define q (make-parameter 2))
(display
  (call/cc
    (lambda (k)
      (parameterize ((p 10) (q 20))
        (k 5)))))
(newline)
(display (p)) (newline)
(display (q)) (newline)

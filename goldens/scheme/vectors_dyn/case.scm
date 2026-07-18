; Heterogeneous vectors (D-077): dyn elements — numbers, strings,
; pairs — filled by tail recursion; the tag-7 tracer's food.
(import (scheme base) (scheme write))
(define (fill! v i)
  (if (< i (vector-length v))
      (begin
        (vector-set! v i (* i i))
        (fill! v (+ i 1)))
      v))
(let ((squares (make-vector 5 0)))
  (fill! squares 0)
  (display (vector-ref squares 4)) (newline))
(let ((mixed (vector "tag" (cons 1 2) 3)))
  (display (vector-ref mixed 0)) (newline)
  (display (car (vector-ref mixed 1))) (newline)
  (set-cdr! (vector-ref mixed 1) 42)
  (display (cdr (vector-ref mixed 1))) (newline))

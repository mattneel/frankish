; Vectors (D-077): TAG_VECTOR = 7 — construction, ref, set!, length;
; aliasing shows the shared array.
(import (scheme base) (scheme write))
(let* ((v (vector 10 20 30)))
  (display (vector-ref v 1)) (newline)
  (vector-set! v 1 99)
  (let ((alias v))
    (display (vector-ref alias 1)) (newline))
  (display (vector-length v)) (newline)
  (let ((w (make-vector 4 7)))
    (display (vector-ref w 3)) (newline)
    (vector-set! w 0 (vector-ref v 2))
    (display (vector-ref w 0)) (newline)))

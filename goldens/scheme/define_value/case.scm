(import (scheme base) (scheme write))
; Top-level value defines (D-081.1): slots in the scm_globals array;
; reads are late-bound at use (f captures nothing — it reads x's slot
; when called), redefinition writes the same slot.
(define (f) x)
(define x 42)
(display (f)) (newline)
(display x) (newline)
(define y (+ x 8))
(display y) (newline)
(define x 99)
(display (f)) (newline)
(display y) (newline)

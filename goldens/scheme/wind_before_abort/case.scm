(import (scheme base) (scheme write))
; An abort raised INSIDE a dynamic-wind BEFORE-thunk skips thunk AND
; after (R7RS: after runs only when before completed). Pre-existing
; native divergence pinned by the M33 panel (D-081.0): the old
; straight-line lowering ran thunk!/after! side effects here.
(display
  (call/cc
    (lambda (k)
      (dynamic-wind
        (lambda () (k 42))
        (lambda () (display "thunk!") (newline) 0)
        (lambda () (display "after!") (newline) 0)))))
(newline)

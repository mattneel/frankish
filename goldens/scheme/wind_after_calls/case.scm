(import (scheme base) (scheme write))
; Thunk aborts; after() calls a user procedure TWICE. Interp/chibi run
; the whole after body; a native guard firing on the in-flight pending
; would truncate it after the first call.
(define (say x) (display x) (newline) x)
(display
  (call/cc
    (lambda (k)
      (dynamic-wind
        (lambda () (display 'in) (newline))
        (lambda () (k 1))
        (lambda () (say 'a) (say 'b))))))
(newline)

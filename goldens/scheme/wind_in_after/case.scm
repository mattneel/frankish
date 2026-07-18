(import (scheme base) (scheme write))
; Thunk aborts; the AFTER thunk contains a full dynamic-wind of its
; own. Its before did NOT abort, so its thunk and after must run.
(display
  (call/cc
    (lambda (k)
      (dynamic-wind
        (lambda () (display 'in) (newline))
        (lambda () (k 1))
        (lambda ()
          (dynamic-wind
            (lambda () (display 'b-in) (newline))
            (lambda () (display 'b-thunk) (newline))
            (lambda () (display 'b-out) (newline))))))))
(newline)

(import (scheme base) (scheme write))
; Plain raise (D-081.4): the handler ESCAPES — the sanctioned exit
; (returning normally from a plain raise is the D-081 trap, chibi's
; secondary exception; corpus law keeps that path out).
(display
  (call/cc
    (lambda (k)
      (with-exception-handler
        (lambda (e) (k 99))
        (lambda () (raise 'boom) (display "unreached"))))))
(newline)

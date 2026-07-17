(import (scheme base) (scheme write))
(define (ask) (raise-continuable 'get))
(define (with-value v thunk)
  (with-exception-handler
    (lambda (e) v)
    thunk))
(display
  (with-value 10
    (lambda ()
      (+ (ask)
         (with-value 20
           (lambda () (+ (ask) (ask))))
         (ask)))))
(newline)

(import (scheme base) (scheme write))
(display
  (call/cc (lambda (k)
    (dynamic-wind
      (lambda () (display 'in) (newline))
      (lambda ()
        (with-exception-handler
          (lambda (e) (k (+ e 40)))
          (lambda ()
            (display 'trying) (newline)
            (raise-continuable 2)
            (display 'unreached) (newline)
            0)))
      (lambda () (display 'out) (newline))))))
(newline)

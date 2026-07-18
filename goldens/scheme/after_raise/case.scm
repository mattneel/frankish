(import (scheme base) (scheme write))
(display
  (call/cc (lambda (k)
    (with-exception-handler
      (lambda (e) (+ e 1))
      (lambda ()
        (dynamic-wind
          (lambda () #f)
          (lambda () (k 42))
          (lambda () (display (raise-continuable 5)) (newline))))))))
(newline)

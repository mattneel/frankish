(import (scheme base) (scheme write))
(display
  (with-exception-handler
    (lambda (e) (+ e 1000))
    (lambda ()
      (with-exception-handler
        (lambda (e) (+ (raise-continuable (+ e 1)) 20))
        (lambda ()
          (+ (raise-continuable 3) 500))))))
(newline)

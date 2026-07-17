(import (scheme base) (scheme write))
(display
  (with-exception-handler
    (lambda (e) (+ e 100))
    (lambda ()
      (display 'start) (newline)
      (let ((r (raise-continuable 5)))
        (display 'resumed) (newline)
        (* r 2)))))
(newline)

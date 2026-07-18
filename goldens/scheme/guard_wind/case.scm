(import (scheme base) (scheme write))
(display
  (guard (e ((eq? e 'ping) (begin (display 'clause) (newline) 'caught)))
    (dynamic-wind
      (lambda () (display 'in) (newline))
      (lambda () (+ 1 (raise-continuable 'ping)))
      (lambda () (display 'out) (newline)))))
(newline)

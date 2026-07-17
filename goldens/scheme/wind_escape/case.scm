(import (scheme base) (scheme write))
(display
  (call/cc (lambda (k)
    (dynamic-wind
      (lambda () (display 'outer-in) (newline))
      (lambda ()
        (dynamic-wind
          (lambda () (display 'inner-in) (newline))
          (lambda () (k 99))
          (lambda () (display 'inner-out) (newline))))
      (lambda () (display 'outer-out) (newline)))))
  )
(newline)

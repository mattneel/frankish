(import (scheme base) (scheme write))
(display (dynamic-wind
  (lambda () (display 'before) (newline) 0)
  (lambda () (display 'during) (newline) 42)
  (lambda () (display 'after) (newline) 0)))
(newline)

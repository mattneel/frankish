(import (scheme base) (scheme write))
(define p (make-parameter 'out))
(parameterize ((p 'in))
  (dynamic-wind
    (lambda () (display (p)) (newline))
    (lambda () (display 'body) (newline))
    (lambda () (display (p)) (newline))))
(dynamic-wind
  (lambda () (display 'pre) (newline))
  (lambda () (parameterize ((p 'deep)) (display (p)) (newline)))
  (lambda () (display (p)) (newline)))

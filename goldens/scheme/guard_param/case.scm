(import (scheme base) (scheme write))
(define p (make-parameter 'outer))
(display
  (guard (e (#t (list e (p))))
    (parameterize ((p 'inner))
      (display (p)) (newline)
      (raise 'boom))))
(newline)

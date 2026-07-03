(import (scheme base) (scheme write))
(define (fact n acc) (if (= n 0) acc (fact (- n 1) (* n acc))))
(display (fact 10 1)) (newline)

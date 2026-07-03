(import (scheme base) (scheme write))
(display
  (call/cc (lambda (k)
    (letrec ((loop (lambda (i)
      (if (= i 5) (k 999) (loop (+ i 1))))))
      (loop 0)))))
(newline)

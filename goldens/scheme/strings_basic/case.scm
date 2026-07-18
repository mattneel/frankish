; Strings (D-077): literals, append, length, equality — the interning
; witness: a DYNAMIC string is string=? to a literal.
(import (scheme base) (scheme write))
(display "hello") (newline)
(let ((s (string-append "ab" "cd")))
  (display s) (newline)
  (display (string-length s)) (newline)
  (display (string=? s "abcd")) (newline)
  (display (string=? s "abce")) (newline))
(display (substring "structured" 0 6)) (newline)
(display (string-append (substring "pairs" 0 4) (substring "mutation" 4 8))) (newline)

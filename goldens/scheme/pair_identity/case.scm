(import (scheme base) (scheme write))
; eq? identity on pairs across bindings and control crossings — the
; mechanism D-081's guard sentinel and flagged raise payloads ride:
; native identity is the heap pointer through the {tag,payload}-word
; abort crossing; interp identity is the shared wrapper Rc.
(let ((p (cons 1 2)))
  (let ((q p)) (display (eq? p q)) (newline))
  (display (eq? p (call/cc (lambda (k) (k p)))))
  (newline)
  (display (eq? p (cons 1 2)))
  (newline)
  (display
    (eq? p
      (call/cc
        (lambda (k)
          (with-exception-handler
            (lambda (e) (k e))
            (lambda () (raise-continuable p)))))))
  (newline)
  (display (+ (car p) 4))
  (newline))

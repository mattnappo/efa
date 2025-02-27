$fib 1:
    load_arg 0
    load_lit 0
    eq
    load_arg 0
    load_lit 1
    eq
    or
    jmp_t L0

    load_arg 0
    load_lit 1
    sub
    call_self

    load_arg 0
    load_lit 2
    sub
    call_self
    add
    ret_val
L0:
    load_arg 0
    ret_val

$main 0:
    load_lit 0
    load_dyn $fib
    call
    ret_val


$func 3:
    load_arg 0
    load_arg 1
    load_arg 2
    add
    add

    ret_val

$main 0:
    .lit 0
    .lit 1
    .lit 2
    .lit 3
    load_lit 1
    load_lit 2
    load_lit 3

    load_dyn $func
    call
    ret_val

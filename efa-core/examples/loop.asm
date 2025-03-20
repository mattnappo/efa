$square 1:
    load_arg 0
    load_arg 0
    mul
    ret_val

$main 0:
    .lit 0
    .lit 5

    load_lit 0
    store_local 0 // i

    load_lit 1
    store_local 1 // n

loop:
    jmp_ge exit

    // loop body
    dbg

    jmp loop

exit:
    load_lit 0
    ret_val


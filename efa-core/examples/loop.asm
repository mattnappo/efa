$square 1:
    load_arg 0
    load_arg 0
    mul
    ret_val

$main 0:
    .lit 0
    .lit 1
    .lit 5

    load_lit 0
    store_loc 0

    load_lit 2
    store_loc 1

    load_lit 0
    store_loc 2

loop_top:
    load_loc 0
    load_loc 1
    jmp_gt exit

    load_loc 0
    load_dyn $square
    call
    load_loc 2
    add
    store_loc 2
    pop

    load_loc 0
    load_lit 1
    add
    store_loc 0

    jmp loop_top

exit:
    load_loc 2
    ret_val


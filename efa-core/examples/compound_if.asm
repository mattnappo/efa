$main 0:
    .lit 0
    .lit 1
    .lit 2
    .lit 3

    # x := 1
    load_lit 1
    store_loc 0

    # y := 2
    load_lit 2
    store_loc 1
    
    # (x == 1) && (y == 2 || y == 3)
    
    load_loc 1
    load_lit 2
    eq

    load_loc 1
    load_lit 3
    eq

    or

    load_loc 0
    load_lit 1
    eq

    and

    jmp_t success

#failure:
#    load_lit 1
#    ret_val

success:
    load_lit 0
    ret_val
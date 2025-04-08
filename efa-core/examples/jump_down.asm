$main 0:
    .lit 0
    .lit true

    load_lit 1
    
    jmp_t exit

exit:

    load_lit 0
    ret_val
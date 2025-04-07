# fast integer square root (binary search method)
$isqrt_fast 1:
    .lit 0
    .lit 1
    .lit 2

    load_lit 0
    store_loc 0  # L=0

    load_lit 0
    store_loc 1  # M=0

    load_lit 1
    load_arg 0
    add
    store_loc 2  # R=arg+1

top:
    # if (L != R - 1)
    load_loc 2
    load_lit 1
    sub
    load_loc 0
    jmp_eq exit

    # loop body

    # M = L+R / 2
    load_loc 0
    load_loc 2
    add
    load_lit 2
    div
    store_loc 1
    

    # if (M * M <= y)
    load_loc 1
    load_loc 1
    mul
    load_arg 0
    jmp_gt right

    # L = M
    load_loc 1
    store_loc 0
    jmp top # escape if

right:
    # R = M
    load_loc 1
    store_loc 2
    jmp top # continue loop

exit:
    # return L
    load_loc 0
    ret_val

$main 0:
    .lit 50000
    .lit 51000
    .lit 1

    load_lit 0
    store_loc 0 # i=50000
    load_lit 1
    store_loc 1 # N=51000

top:
    load_loc 0
    load_loc 1
    jmp_ge exit

    # call
    load_loc 0
    load_dyn $isqrt_fast
    call
    #dbg
    store_loc 2 # store into last_soln
    pop

    # i++
    load_loc 0
    load_lit 2
    add
    store_loc 0
    jmp top

exit:
    load_loc 2
    ret_val


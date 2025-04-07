$square 1:
    load_arg 0
    load_arg 0
    mul
    ret_val

# TODO: NOT FINISHED

$main 0:
    .lit 0
    .lit 10
    .lit 1
    .lit 2
    .lit 2
    .lit 4

    load_lit 3
    load_lit 4
    load_lit 5
    cont_make 3
    store_loc 1 # store the array in x1

    load_lit 0
    store_loc 0    # i = 0

    load_loc 1
    cont_len
    store_loc 2    # n = len(arr)

top:
    load_loc 0
    load_loc 2
    jmp_ge exit

    load_loc 1 # arr, for the cont_set
    
    # get arr[i]
    load_loc 1 # arr, for the cont_get
    load_loc 0 # i
    cont_get
    # square it
    load_dyn $square
    call

    # store back into arr
    load_loc 0 # i
    cont_set
    store_loc 1 # x1 <- new_arr
    
    # inc i
    load_lit 2 # 1
    load_loc 0
    add
    store_loc 0
    
    jmp top

exit:
    load_loc 1
    dbg
    pop

    load_lit 0
    ret_val
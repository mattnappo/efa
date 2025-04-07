$main 0:
    .lit 0
    .lit 1
    .lit 2
    .lit 3
    .lit 4
    .lit 5
    .lit 6

    load_lit 1
    load_lit 2
    load_lit 3
    cont_make 3

    load_lit 4
    load_lit 5
    load_lit 6
    cont_make 3
    
    cont_make 2 # make 2d

    # get 6

    load_lit 1 # second list
    cont_get

    load_lit 2 # last element
    cont_get

    ret_val     # return 6

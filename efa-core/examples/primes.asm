$biggest_prime_under 1:
    .lit 2
    .lit true
    .lit false
    .lit 0
    .lit 1

    load_lit 0   # i
    store_loc 0

top:
    # while i < n:
    load_loc 0   # i
    load_arg 0   # n
    jmp_ge exit

    load_lit 1  # true
    store_loc 1 # is_prime = true
    
    load_lit 0  # 2
    store_loc 2 # j = 2
jlti:
    # while j < i:
    load_loc 2 # j
    load_loc 0 # i
    jmp_ge btm

    # i % j
    load_loc 0 # i
    load_loc 2 # j
    mod

    load_lit 3 # 0
    
    # i % j == 0
    jmp_ne fail
    load_lit 2
    store_loc 1  # is_prime = false
    
fail:
    # j += 1
    load_loc 2 # j
    load_lit 4 # 1
    add
    store_loc 2
    jmp jlti

btm:
    # check if prime
    load_loc 1 # is_prime
    jmp_f inc
    # i is prime
    load_loc 0 # i
    load_loc 0
    store_loc 3 # store into last_prime
    dbg
    pop
inc:
    #i += 1
    load_loc 0
    load_lit 4
    add
    store_loc 0
    jmp top

exit:
    load_loc 3
    ret_val

$main 0:
    .lit 0
    .lit 100
    
    load_lit 1
    load_dyn $biggest_prime_under
    call

    ret_val

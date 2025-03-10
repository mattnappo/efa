$foo 0:
    load_dyn $bar
    call

    ret

$bar 0:
    load_dyn $baz
    call

    load_dyn $cap
    call
    ret

$baz 0:
    nop
    ret

$cap 0:
    nop
    ret

$main 0:
    load_dyn $foo
    call

    call_self

    #lit 0
    load_lit 0
    ret_val

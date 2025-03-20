$foo 0:
    load_dyn $bar
    call

    ret_val

$bar 0:
    load_dyn $baz
    call

    load_dyn $cap
    call
    ret_val

$baz 0:
    nop
    ret

$cap 0:
    .lit 7
    load_lit 0
    ret_val

$main 0:
    load_dyn $foo
    call

    ret_val

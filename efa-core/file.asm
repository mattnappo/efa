$cap 0:
    .lit 7
    load_lit 0
    ret
$bar 0:
    load_func 0x9a1b03a976773bb8bab9bebccfa18d6d
    call
    load_func 0xf26e0a3f3c42b95620dd3261197f6b07
    call
    ret
$baz 0:
    nop
    ret
$foo 0:
    load_func 0x614537e288ef594dbe63ad742797afe1
    call
    ret
$main 0:
    load_func 0xba7e9eb7b088440a4a20f0ec602dd49e
    call
    ret

# 0xf26e0a3f3c42b95620dd3261197f6b07
$cap 0:
    .lit 7
    load_lit 0
    ret

# 0x614537e288ef594dbe63ad742797afe1
$bar 0:
    load_func 0x9a1b03a976773bb8bab9bebccfa18d6d
    call
    load_func 0xf26e0a3f3c42b95620dd3261197f6b07
    call
    ret

# 0x9a1b03a976773bb8bab9bebccfa18d6d
$baz 0:
    nop
    ret

# 0xba7e9eb7b088440a4a20f0ec602dd49e
$foo 0:
    load_func 0x614537e288ef594dbe63ad742797afe1
    call
    ret

# 0xc1f0f4e977bcac23fc277497c583ff16
$main 0:
    load_func 0xba7e9eb7b088440a4a20f0ec602dd49e
    call
    ret


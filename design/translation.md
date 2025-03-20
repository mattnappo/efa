# Translation Notes

## For loops

Example:


```
n = 5
sum = 0
for (i = 0; i < n; i++)
    t = square i
    sum = sum + t
```

which is just

```
i = 0
n = 5
sum = 0
L0:
    check i < n
    if false: goto L1
    sum += square i
    i += 1
    goto L0
L1:
    ret_val

```

in stack based:
```
    push lit0 (sum)
    push lit0 (i)
    push lit5 (n)
loop_top:
    jmp_ge exit

exit:
    
```


push 0




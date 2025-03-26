# Lists

## Building Lists

### `make_list n`

pop n elements from the stack and push a list onto the stack
* n=0 case will build an empty list
* infers type of the list from the type of `stack.top()`
* will crash if all `n` elements are not of the same type

## Head/Tail/Ext

### `list_head`

get the head of a list
  * `list_head [A; B; C; ...] --> A`


### `list_tail`

get the tail of a list
  * `list_tail [A; B; C; D; ...] --> [B; C; D; ...]`


### `list_ext`

extend a list with another list
1. pop the top two stack elements B(top), then A, which must be lists
2. push a list B + A onto the stack

```
tos --> [ C; D ]
        [ A; B ]
```
will push `[A; B; C; D]`

### `list_append`
Thinking about whether I actually want this instruction or not...

```
val = stack.pop()
A = stack.pop()
push A + [val]     # Just extension with new singleton list
```

### `list_insert <i>`

if `i` supplied:
```
val = stack.pop()
A = stack.pop()
B = insert(A, i, val)
push B
```

if `i` not supplied:
```
i = stack.pop()
list_insert i
```

### `list_get i`

Push `A[i]` onto the stack, where `A` is a list on tos, then pop A
* Err if oob

### `list_get`

```
i = stack.pop()
A = stack.pop()
push A[i]
```

### `list_set`

```
val = stack.pop()
i = stack.pop()
A = stack.pop()
A[i] = val
```

### `list_len`
```
A = list.pop()
push len(A)
```

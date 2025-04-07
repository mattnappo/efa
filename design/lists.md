# Lists, Tuples, Structs

Lists (arrays), tuples, and structs have the exact same representation, as a `Value::Container`. See `memory-representation.md` for more details.

## Building Containers

### `make_cont n`

pop n elements from the stack and push a cont onto the stack
* n=0 case will build an empty cont
* infers type of the cont from the type of `stack.top()`
* will crash if all `n` elements are not of the same type

## Head/Tail/Extend

### `cont_head`

get the head of a container
  * `cont_head [A; B; C; ...] --> A`


### `cont_tail`

get the tail of a container
  * `cont_tail [A; B; C; D; ...] --> [B; C; D; ...]`


### `cont_ext`

extend a cont with another cont
1. pop the top two stack elements B(top), then A, which must be containers
2. push a cont B + A onto the stack

```
tos --> [ C; D ]
        [ A; B ]
```
will push `[A; B; C; D]`

### `cont_append`
Thinking about whether I actually want this instruction or not...

```
val = stack.pop()
A = stack.pop()
push A + [val]     # Just extension with new singleton list
```

### `cont_insert <i>`

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
cont_insert i
```

### `cont_get i`

Push `A[i]` onto the stack, where `A` is a cont on tos, then pop A
* Err if oob

### `cont_get`

```
i = stack.pop()
A = stack.pop()
push A[i]
```

### `cont_set`

```
val = stack.pop()
i = stack.pop()
A = stack.pop()
A[i] = val
```

### `cont_len`
```
A = cont.pop()
push len(A)
```

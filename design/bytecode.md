# The Bytecode

The bytecode is stack based, and is largely based on the bytecodes of Python and Java. 

This is a fixed instruction size bytecode. Each opcode is two bytes, followed by a two-byte argument, making for 4-byte instructions. The only "exception" to this is the LOAD_FUNC and LOAD_TYPE instructions, which are followed by a 32-byte hash literal.

Every bytecode fragment is assumed to live within a single code object. This means that, for example, `JUMP` can only jump within the same code object.

# Opcodes

## Stack Manipulation and Variables
* LOAD_ARG(i) - Push the `i`th function argument onto the stack.
* LOAD_LOCAL(i) - Push the `i`th local variable onto the stack.
* LOAD_CONST(i) - Push the `i`th constant code object onto the stack. Literals in source code are stored in a table of constants.
* STORE_LOCAL(i) - Set `locals[i] = pop()`.
* POP - Remove the value at the top of the stack.

## Functions
* LOAD_FUNC(hash) - Push the 32-byte hash literal for a function onto the stack.
* CALL - Call a function by its hash at the top of the stack.
* RETURN - Return from function.
* RETURN_VAL - Return from function and return the top of the stack.

## Control Flow
* JUMP - Jump to the bytecode address at the top of the stack.
* JUMP_EQ
* JUMP_GT
* JUMP_GE
* JUMP_LT
* JUMP_LE

## Arithmetic
Binary
* ADD
* MUL
* DIV
* SUB
* AND
* OR
* XOR

Unary
* NOT - `push(-pop())`
* NEG - `push(~pop())`

## Arrays
From top to bottom: end, start, key, container, value.

* LOAD_ARRAY - Push `container[key]` to the top of the stack.
* STORE_ARRAY - Set `container[key] = value`.
* MAKE_ARRAY - Push a new array to the top of the stack with `pop()` elements.
* MAKE_SLICE - Make a new array `container[start:end]` and push it to the top of the stack.
* STORE_SLICE - Set `container[start:end] = value`.

## Types

* LOAD_TYPE(hash) - Push the 32-byte hash literal for a type onto the stack. Similar to LOAD_FUNC.

### Structs / Enums
* LOAD_FIELD(i) - Push the `i`th field of the struct at the top of the stack to the top of the stack.
* STORE_FIELD(i) - Set `struct[i] = pop()` where `struct` is the struct at the top of the stack.
* MAKE_STRUCT - Pop from the stack to set the fields of a new struct until a type hash is found. Push a new struct of the type supplied by the hash to the top of the stack.

### Enums
TODO

### Tuples
TODO

## Other
* NOP

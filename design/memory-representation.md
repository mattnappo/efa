# In-Memory Representation of Values

Currently the `Value` type doesn't allow for much flexibility; there is only support for primitive types. In order to get compound types like tuples, lists, and records (structs), we need a more complex type.

## OCaml-Inspired "blocks"

A value on the stack, defined recursively, is a type that is either a primitive (including strings) or a list of values

`type value = int | char | bool | string | ... | value list`

```
enum Value<'a> {
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    I128(i128),
    U128(u128),
    Isize(isize),
    Usize(usize),

    F32(f32),
    F64(f64),

    Char(char),
    Bool(bool),

    Str(&'a str),
    OwnedStr(&'a str),

    Container(Vec<Value<'a>>)
}
```

Containers are used to hold structs, lists, tuples, and arrays.

### Example: Array of integer tuples

How would the value `[(1, 2, 3), (4, 5, 6)]` be stored?

```rust
Value::Container(vec![
    Value::Container(vec![Value::I32(1), Value::I32(2), Value::I32(3)]),
    Value::Container(vec![Value::I32(4), Value::I32(5), Value::I32(6)]),
]);

```

### Example: structs

How could we represent the following struct

```rust
struct Person1 {
    id: usize,
    age: i32,
    is_male: bool,
}

Person1 {1, 21, true}
```

```rust
Value::Container(vec![Value::Usize(1), Value::I32(21), Value::Bool(true)])
```

What about this struct?

```rust
struct Person2 {
    id_hash: Vec<u8>,
    age: i32,
    is_male: i32,
}

Person2 {
    id_hash: vec![2, 4, 6, 8],
    21,
    true,
}
```

Value::Container(vec![Value::Usize(1), Value::I32(21), Value::Bool(true)])

## Names and types

The compiler is entirely responsible for emitting code that correctly handles array index access versus struct field access. To the bytecode, they are all the same. The compiler is also entirely responsible for transforming struct field name accesses into indices. The names are completely erased at runtime.

No knowledge of types is necessary at runtime, which eventually can be optimized away to store everything as raw bytes (might be tricky to do this without unsafe rust, and would also probably want alignment). The compiler will typecheck everything so that emitted bytecode is type-semantically valid.


# Enums

Enums will be compiled down to tagged unions.

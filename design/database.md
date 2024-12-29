# The Code Database

There are a few key maps that must be maintained. The first map keeps track of what textual names actually refer to. This map is based on the idea that textual names should merely serve as a soft link to a concrete piece of code.

## Name Map: `Names -> Hashes`

A `Name` is a string with some additional metadata that represents the name of a function, type, or module.

The Name Map is used to keep track of what names point to what hashes. Thus, `NameMap[name]` is the hash that the code tool currently thinks `name` should point to.

### Example
User code
```rust
1 struct T(u32)
2 fn main() {
3     let t = T(7);
4 }
```

The usage of `T` in line 3 must be resolved. If line 1 is the most up-to-date definition of `T`, then `NameMap['T']` will point to whatever the hash of the `T` definition on line 1 is.

This map can be changed by the code tool for refactoring.

## Code Map: `Hashes -> CodeObjects`

This map identifies each particular hash with the piece of code that the hash refers to. CodeObjects are described in more detail in the interpreter design doc. This map is the final map consulted when functions are called. It determines what code a hash actually refers to.

## Type Map: `Hashes -> Types`

A `Type` stores information about a type, such as whether the type is a struct, enum, or union, and how members of the type should be accessed. Thus, the domain of this map is the same keyspace as the Code Map, which is the same as the codomain of the Name Map. 

```rust
enum Type {
    Struct(StructTy),
    Enum(EnumTy),
    Union(UnionTy)
}

struct StructTy {
    fields: BTreeMap<Name, (Type, Value)>
}
```

### Alternative struct implementation

Make a macro to create `StructN` where `N` ranges from 0 to 128.
```rust
struct Struct4<T0, T1, T2, T3> {
    f0: T0,
    f1: T1,
    f2: T2,
    f3: T3,
}
...
struct StructN<T0, T1, ..., TN> {
    f0: T0,
    f1: T1,
    ...
    fN: TN,
}
trait Struct;
impl Struct for Structi {} // For all i in 1..N
```

Then the `enum Type` has `Struct` variant `Struct(Box<dyn Struct>)`

See [the playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=4d5631cdb8868e0861f9d4ae6a6ef5a3).

Could make this even better by using unit structs so that the fields are unnamed.

# Other Notes
* SHA-256 is used for computing the hash of bytecode and types.
* The Code Map and Type Map are, for the most part, immutable and append-only.
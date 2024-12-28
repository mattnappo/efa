# The Code Database

There are a few key maps that must be maintained. The first map keeps track of what textual names actually refer to. This map is based on the idea that textual names should merely serve as a soft link to a concrete piece of code.

## Name Map: `Names -> Hashes`

A `Name` is a string with some additional metadata that represents the name of a function, type, module, or constant. 

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

## Code Map: `Hashes -> CodeObject`

This map identifies each particular hash with the piece of code that the hash refers to. A CodeObject is essentially just a chunk of readily executable bytecode.

## Type Map: `Hashes -> Types`

A `Type` stores information about types such as whether the type is a struct, enum, or union, and how members of the type should be accessed. Thus, the domain of this map is in the same keyspace as the Code Map, which is the same as the codomain of the Name Map. 

```rust
enum Type {
    Struct(StructTy),
    Enum(EnumTy),
    Union(UnionTy)
}

struct StructTy {
    fields: Vec<(Type, Value)>
}

```
# Other Notes
SHA-256 is used for all hashing.
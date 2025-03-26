# Notes

* Might want to throw an error if bytecode containing a `load_dyn` is ever hashed.
* First thing to optimize: copying stack values. Could use a `Box` or keep references in struct / have an `Owned` variant of lists/sets/maps

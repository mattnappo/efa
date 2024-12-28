# The Bytecode Interpreter

## Code Objects

A `CodeObject` contains a block of executable bytecode with additional metadata required for execution. This is based on Python's [codeobject](https://docs.python.org/3/reference/datamodel.html#index-60).

Fields:
* co_consts - all literals in the function

## Call Stack and Frames


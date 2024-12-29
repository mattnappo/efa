# The Bytecode Interpreter

## Code Objects

A `CodeObject` contains a block of executable bytecode with additional metadata required for execution. This is based on Python's [codeobject](https://docs.python.org/3/reference/datamodel.html#index-60).

Fields:
* name - Name of the function for debugging.
* hash - Hash of the CodeObject.
* consts - All literals in the function.
* argcount - Number of arguments.
* locals - Array of local variable names. Values are stored in a separate map during interpretation.
* code - The bytecode to execute.

## Call Stack and Frames

The interpreter state maintains a fixed size call stack. Each entry is of type `StackFrame`, and contains a CodeObject to execute. Each `StackFrame` also has a mapping of local variables to their values. 

## Interpreter

Maintains a call stack, data stack ("the stack"), and instruction pointer. Executes instructions, keeps track of local variables, global constants, and function calls, and consults the database for hash resolutions.
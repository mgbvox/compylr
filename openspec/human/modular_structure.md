Compylr should take spiritual inspiration from LLVM - particularly in where it comes to modularity.

LLVM follows a frontend - IR - backend design (loosely speaking), where both the frontend and backend
can be independently implemented in/for the target/source language of one's choosing.

Currently, we have implemented a basic python -> IR -> rust transpilation flow; however, certain python-specific
semantics are baked into the frontend and in the generated rust code (see @demo/.compylr/crate/src/compat.rs and bindings.rs)

These do not easily allow the extension of the frontend or backend to our other target supported languages (e.g. c++, go).

Analyze and then propose a refactoring of what we currently have that targets a maximally modular 
interface design that any language in our supported list (which may change)
can implement.

The general structure, for N supported languages:

For Lang X -> Lang Y:
language: purpose

X: frontend, binds Rust main
Rust: X->IR lowering (N crates)
Rust: core: 
    * agnostic IR optimizations (configurable)
    * X->Y optimizations (still on the IR)
Rust: IR -> Y generation (N crates)
Y: post-generation Y-specific optimizations (if compatible with expectations from X or explicitly allowed)

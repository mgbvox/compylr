Currently, generated Rust lives entirely in a lib.rs file. This makes a human reading JUST the generated
rust have to search for a bit before finding things.

Ergonomics update:

Generated structure should follow:

```
crate/
    src/
        lib.rs # top level, keep lean
        generated.rs # ONLY generated code
        compat.rs # all compatibility logic, e.g. python behaviors in rust like py_add, needed for the generated code
```

Feel free to break compat.rs up into smaller modules if it makes sense to do so by concerns.
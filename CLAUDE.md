# Target End State

I `uv add compylr` in a project.
Then, in my code:
```python
import compylr

@compylr.compyle
def my_cool_function[T, U, V](a:T, b:U) -> V:
    ... # logic
```

Under the hood:
* first run: transpile my_cool_function to rust with python bindings via maturin, install in the project venv
* subsequent runs: usage of my_cool_function is imported from the rust bindings and replaced by the decorator at runtime.

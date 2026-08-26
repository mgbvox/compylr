"""Recovering a function's source from the live function object.

The decorator never sees a file path. `inspect.getsource` hands back the text as written, which
means two things have to be undone before it is compilable on its own: the decorator lines that
triggered this in the first place, and any indentation from an enclosing scope.
"""

from __future__ import annotations

import ast
import inspect
import textwrap
from collections.abc import Callable
from typing import Any

__all__ = ["capture_source"]


def capture_source(function: Callable[..., Any]) -> str:
    """Return `function`'s source as a standalone, top-level definition.

    Raises `OSError` when the source is unavailable — a function defined in a REPL or built by
    `exec` has no retrievable text, and there is nothing compylr can do with it.
    """
    raw = inspect.getsource(function)
    # A function defined inside another function or a class body arrives indented, which is a
    # syntax error on its own.
    dedented = textwrap.dedent(raw)
    return _strip_decorators(dedented)


def _strip_decorators(source: str) -> str:
    """Remove decorator lines preceding the definition.

    The `@c.compyle` line is not part of the member being compiled, and leaving it in would make
    the source fail lowering on a construct the user did not write for the compiler.

    Parsing rather than scanning for `def` or `class`: a decorator can span several lines and can
    contain a string holding either word, so the AST is the only thing that knows where the
    decorators actually end.
    """
    try:
        tree = ast.parse(source)
    except SyntaxError:
        # Let the compiler report it, with a location, rather than guessing here.
        return source

    if not tree.body:
        return source
    node = tree.body[0]
    if (
        not isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef)
        or not node.decorator_list
    ):
        return source

    lines = source.splitlines(keepends=True)
    # `node.lineno` is the `def` or `class` line even when decorators precede it, and it is
    # 1-based.
    return "".join(lines[node.lineno - 1 :])

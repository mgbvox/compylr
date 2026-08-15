def alias_parameter(a: int) -> int:
    b = a
    return b


def alias_local() -> str:
    x: str = "hi"
    y = x
    return y


def alias_chain(a: bool) -> bool:
    b = a
    c = b
    return c


def annotated_alias(a: int) -> int:
    b: int = a
    return b

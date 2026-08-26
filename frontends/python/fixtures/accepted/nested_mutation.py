def zeros(rows: int, columns: int) -> list[list[int]]:
    table: list[list[int]] = []
    for _row in range(rows):
        line: list[int] = []
        for _column in range(columns):
            line.append(0)
        table.append(line)
    return table


def diagonal(size: int) -> list[list[int]]:
    table = zeros(size, size)
    for i in range(size):
        table[i][i] = 1
    return table


def bucket(key: str, value: int) -> dict[str, list[int]]:
    buckets: dict[str, list[int]] = {}
    row: list[int] = []
    buckets[key] = row
    buckets[key].append(value)
    return buckets


class Grid:
    def __init__(self, size: int) -> None:
        self.rows: list[list[int]] = []
        for _row in range(size):
            line: list[int] = []
            for _column in range(size):
                line.append(0)
            self.rows.append(line)

    def write(self, row: int, column: int, value: int) -> None:
        self.rows[row][column] = value

    def read(self, row: int, column: int) -> int:
        return self.rows[row][column]

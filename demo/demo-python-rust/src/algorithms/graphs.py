"""Graph traversal over an adjacency mapping.

A graph is a `dict[int, list[int]]` — nested collections are supported to any depth — and every
algorithm here is written against two constraints that shape all graph code in the subset.

**There is no `pop`.** A queue is a list plus a read cursor, and a stack is a list plus a `top`
index that overwrites rather than shrinks. The queue version is not a workaround at all: a cursor
is O(1) where `list.pop(0)` is O(n), so it is what the interpreted original should have been.

**Iterating a mapping yields its keys in no guaranteed order**, and the order varies between
runs. Any function whose *result* is an ordered thing must therefore impose the order itself —
which is why `node_list` sorts, and why `topological_order` always takes the smallest ready node
instead of the first one the mapping offered.
"""

from __future__ import annotations

from ._compylr import c
from .sorting import merge_sort


@c.compyle
def node_list(graph: dict[int, list[int]]) -> list[int]:
    """Every node the graph mentions — keys and neighbours alike — in ascending order.

    Note the annotation on `ordered`. `merge_sort` lives in another module, so at the moment this
    function is validated its signature is not visible and the binding's type cannot be inferred.
    Rejecting the call instead would make whether your code compiles depend on which function you
    happened to decorate first; requiring the annotation does not. The call is still checked —
    once every source is assembled into one unit.
    """
    seen: dict[int, int] = {}
    raw: list[int] = []
    for node in graph:  # noqa: PLC0206 - no `.items()` in the subset
        if node not in seen:
            seen[node] = 1
            raw.append(node)
        for neighbour in graph[node]:
            if neighbour in seen:
                continue
            seen[neighbour] = 1
            raw.append(neighbour)
    ordered: list[int] = merge_sort(raw)
    return ordered


@c.compyle
def bfs_distances(graph: dict[int, list[int]], start: int) -> dict[int, int]:
    """How many hops each reachable node is from `start`.

    The queue is a list and a `head` cursor. Nothing is ever removed, so the list is also the
    visit order if you want it — and the traversal never pays `pop(0)`'s cost of shifting every
    remaining element down by one.
    """
    distance: dict[int, int] = {}
    distance[start] = 0
    queue: list[int] = []
    queue.append(start)
    head = 0
    while head < len(queue):
        node = queue[head]
        head = head + 1
        if node not in graph:
            continue
        for neighbour in graph[node]:
            if neighbour in distance:
                continue
            distance[neighbour] = distance[node] + 1
            queue.append(neighbour)
    return distance


@c.compyle
def depth_first_order(graph: dict[int, list[int]], start: int) -> list[int]:
    """Nodes in the order a depth-first traversal first reaches them.

    The stack is a list and a `top` index: pushing writes over a slot that is past `top` if one
    exists and appends otherwise, and popping just moves `top` down. The list therefore only ever
    grows to the deepest the stack ever was, which is the allocation `pop` would have given back.

    Neighbours go on in reverse so the first one listed is the first one visited, which is what
    the recursive version does and what a reader will expect.
    """
    order: list[int] = []
    seen: dict[int, int] = {}
    stack: list[int] = []
    stack.append(start)
    top = 1
    while top > 0:
        top = top - 1
        node = stack[top]
        if node in seen:
            continue
        seen[node] = 1
        order.append(node)
        if node not in graph:
            continue
        neighbours = graph[node]
        index = len(neighbours) - 1
        while index >= 0:
            if top < len(stack):
                stack[top] = neighbours[index]
            else:
                stack.append(neighbours[index])
            top = top + 1
            index = index - 1
    return order


@c.compyle
def topological_order(graph: dict[int, list[int]]) -> list[int]:
    """An order in which every edge points forward. Empty when the graph has a cycle.

    Kahn's algorithm, always taking the **smallest** ready node rather than the first the mapping
    offered — `node_list` is ascending, so scanning it in order does that. Without the rule this
    would return a different valid order on different runs of the same program, and a test that
    pinned one of them would be flaky rather than the compiler being wrong.

    An empty result is ambiguous for an empty graph, which has no order to return either way.
    `has_cycle` is the unambiguous question.
    """
    nodes = node_list(graph)
    indegree: dict[int, int] = {}
    for node in nodes:
        indegree[node] = 0
    for node in nodes:
        if node not in graph:
            continue
        for neighbour in graph[node]:
            indegree[neighbour] = indegree[neighbour] + 1

    order: list[int] = []
    placed: dict[int, int] = {}
    while len(order) < len(nodes):
        ready = False
        chosen = 0
        for node in nodes:
            if node in placed:
                continue
            if indegree[node] == 0:
                chosen = node
                ready = True
                break
        if ready:
            placed[chosen] = 1
            order.append(chosen)
            if chosen in graph:
                for neighbour in graph[chosen]:
                    indegree[neighbour] = indegree[neighbour] - 1
        else:
            return []
    return order


@c.compyle
def has_cycle(graph: dict[int, list[int]]) -> bool:
    """Whether the graph contains a directed cycle.

    Asked of `topological_order`'s result rather than by a second traversal: a graph orders
    completely exactly when it is acyclic, so one implementation answers both questions and the
    two can never disagree.
    """
    return len(topological_order(graph)) < len(node_list(graph))

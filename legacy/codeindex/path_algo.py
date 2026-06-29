import time
from collections import defaultdict

class PathFinder:
    def __init__(self, conn):
        self.conn = conn

    def db_callees_batch(self, nodes: set[str]) -> dict[str, list[tuple[str, str]]]:
        """1 SQL query cho toàn bộ frontier — thay vì N queries (N = frontier size)."""
        if not nodes:
            return {}
        placeholders = ",".join("?" * len(nodes))
        rows = self.conn.execute(
            f"SELECT from_symbol, to_symbol, edge_confidence "
            f"FROM call_edges WHERE from_symbol IN ({placeholders})",
            list(nodes)
        ).fetchall()
        result: dict[str, list] = defaultdict(list)
        for from_s, to_s, conf in rows:
            result[from_s].append((to_s, conf))
        return dict(result)

    def db_callers_batch(self, nodes: set[str]) -> dict[str, list[tuple[str, str]]]:
        """1 SQL query cho toàn bộ backward frontier. Uses idx_call_edges_to."""
        if not nodes:
            return {}
        placeholders = ",".join("?" * len(nodes))
        rows = self.conn.execute(
            f"SELECT to_symbol, from_symbol, edge_confidence "
            f"FROM call_edges WHERE to_symbol IN ({placeholders})",
            list(nodes)
        ).fetchall()
        result: dict[str, list] = defaultdict(list)
        for to_s, from_s, conf in rows:
            result[to_s].append((from_s, conf))
        return dict(result)

    def bidirectional_bfs_path(
        self, from_sym: str, to_sym: str,
        max_hops: int, max_paths: int, timeout_ms: int
    ) -> tuple[list[list[tuple[str, str | None]]], bool | None, str | None]:
        """
        Returns: (routes, exists, terminated_by)
        routes: list of paths, mỗi path là list of (symbol, incoming_edge_confidence).
        """
        if from_sym == to_sym:
            return [[(from_sym, None)]], True, None   # self-loop: length = 0 (0 edges traversed)

        start = time.monotonic()
        deadline = start + timeout_ms / 1000

        forward_pred:  dict[str, tuple | None] = {from_sym: None}
        backward_pred: dict[str, tuple | None] = {to_sym:   None}
        forward_frontier:  set[str] = {from_sym}
        backward_frontier: set[str] = {to_sym}

        f_depth = 0
        b_depth = 0
        meeting_nodes: set[str] = set()   # [F3] set thay vì list

        # [F10] Exhaustion tracked explicitly — KHÔNG suy ra từ len(frontier) == 0.
        forward_exhausted = False
        backward_exhausted = False

        # [F10] Counter tie riêng — chỉ flip khi THỰC SỰ vào nhánh tie.
        tie_toggle = True

        while not (forward_exhausted and backward_exhausted):
            if time.monotonic() > deadline:
                return [], None, "timeout"
            if f_depth + b_depth >= max_hops:
                return [], None, "max_hops"

            # [F2+F10] Vertex-balanced: expand smaller frontier.
            # Khi equal → alternate qua tie_toggle (chỉ flip trong nhánh tie này).
            if forward_exhausted:
                expand_forward = False
            elif backward_exhausted:
                expand_forward = True
            elif len(forward_frontier) < len(backward_frontier):
                expand_forward = True
            elif len(backward_frontier) < len(forward_frontier):
                expand_forward = False
            else:
                expand_forward = tie_toggle
                tie_toggle = not tie_toggle

            if expand_forward:
                callee_map = self.db_callees_batch(forward_frontier)   # [F1] batch
                new_f: set[str] = set()
                for node in forward_frontier:
                    for callee, edge in callee_map.get(node, []):
                        if callee not in forward_pred:
                            forward_pred[callee] = (node, edge)
                            new_f.add(callee)
                            if callee in backward_pred:
                                meeting_nodes.add(callee)
                forward_frontier = new_f
                if not forward_frontier:
                    forward_exhausted = True       # [F10]
                else:
                    f_depth += 1
            else:
                caller_map = self.db_callers_batch(backward_frontier)   # [F1] batch
                new_b: set[str] = set()
                for node in backward_frontier:
                    for caller, edge in caller_map.get(node, []):
                        if caller not in backward_pred:
                            backward_pred[caller] = (node, edge)
                            new_b.add(caller)
                            if caller in forward_pred:
                                meeting_nodes.add(caller)
                backward_frontier = new_b
                if not backward_frontier:
                    backward_exhausted = True      # [F10]
                else:
                    b_depth += 1

            if meeting_nodes:
                break

        if not meeting_nodes:
            return [], False, None  # cả hai phía exhausted — exists: false, chắc chắn

        routes: list[list[tuple[str, str | None]]] = []
        for meeting in meeting_nodes:
            fwd: list[tuple[str, str | None]] = []
            node = meeting
            while node is not None:
                pred = forward_pred[node]
                fwd.append((node, pred[1] if pred else None))
                node = pred[0] if pred else None
            fwd.reverse()

            bwd: list[tuple[str, str | None]] = []
            node = meeting
            while True:
                pred = backward_pred.get(node)
                if pred is None:
                    break
                next_node, edge = pred
                bwd.append((next_node, edge))
                node = next_node

            routes.append(fwd + bwd)
            if len(routes) >= max_paths:
                break

        terminated_by = "path_count" if len(routes) >= max_paths else None
        return routes, True, terminated_by

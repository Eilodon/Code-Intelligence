import subprocess
from pathlib import Path


def get_git_diff(
    project_root: Path,
    *,
    staged: bool = False,
    commits: str | None = None,
    timeout: float = 10.0,
) -> tuple[str | None, str | None]:
    """
    Run git diff and return (diff_text, error_message).

    -M flag enables rename detection (content-similarity based).
    Architecture spec: staged → 'git diff --cached -M'; commits → 'git diff -M <range>'.
    Returns (None, error_msg) when git is unavailable or returns non-zero exit.
    """
    if staged:
        cmd = ["git", "diff", "--cached", "-M"]
    elif commits is not None:
        cmd = ["git", "diff", "-M", commits]
    else:
        return None, "Provide exactly one of staged=True or commits=<range>."

    try:
        result = subprocess.run(
            cmd, cwd=project_root,
            capture_output=True, text=True, timeout=timeout,
        )
        if result.returncode != 0:
            return None, result.stderr.strip() or f"git exited {result.returncode}"
        return result.stdout, None
    except FileNotFoundError:
        return None, "git not found in PATH"
    except subprocess.TimeoutExpired:
        return None, f"git diff timed out after {timeout}s"


def get_signature_range(symbol_node) -> tuple[int, int]:
    params_node = symbol_node.child_by_field_name("parameters")
    return_type_node = symbol_node.child_by_field_name("return_type")
    end_node = return_type_node or params_node or symbol_node
    return (symbol_node.start_point[0] + 1, end_node.end_point[0] + 1)

def is_signature_changed(signature_range, hunk_ranges) -> bool:
    sig_start, sig_end = signature_range
    return any(not (hunk_end < sig_start or hunk_start > sig_end)
               for hunk_start, hunk_end in hunk_ranges)

def compute_aggregate_risk(affected_symbols: list, unindexed_files: list) -> str:
    RISK_ORDER = {"low": 0, "medium": 1, "high": 2, "critical": 3}
    aggregate_risk = max(
        (s["risk_assessment"]["level"] for s in affected_symbols),
        key=lambda level: RISK_ORDER[level],
        default="low"
    )
    if unindexed_files:
        aggregate_risk = "unknown"
    return aggregate_risk

def escalate_risk_if_signature_changed(affected_symbol: dict, level: str, reasons: list) -> str:
    RISK_ORDER = {"low": 0, "medium": 1, "high": 2, "critical": 3}
    if affected_symbol.get("signature_changed"):
        reasons.append("signature modified — all call sites may need update")
        if RISK_ORDER[level] < RISK_ORDER["high"]:
            level = "high"
    return level

def sort_affected_symbols(affected_symbols: list, max_affected_symbols: int) -> list:
    RISK_ORDER = {"low": 0, "medium": 1, "high": 2, "critical": 3}
    affected_symbols.sort(
        key=lambda s: (
            RISK_ORDER[s["risk_assessment"]["level"]],  # higher risk first
            1 if s.get("signature_changed") else 0,      # signature changes first
            s.get("blast_radius", {}).get("direct_callers", 0),  # higher blast radius first
        ),
        reverse=True
    )
    return affected_symbols[:max_affected_symbols]

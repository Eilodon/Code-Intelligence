from .coverage_reader import CoverageData

def _compute_dead_code_confidence(
    symbol_path: str,
    line_start: int,
    line_end: int,
    caller_count: int,
    is_entry_point: bool,
    is_private: bool,
    # is_private depends on language logic, e.g., name starts with _, or unexported Go function
    scope_clear: bool,
    # scope_clear: True if analyzer could confidently determine scope (e.g. Python class methods).
    #   False if it's dynamic/duck-typed and could be called from anywhere.
    #   Default:    False (conservative — scope unclear → lower dead_code_confidence)
    coverage: CoverageData,
) -> tuple[str, str]:
    """
    Returns (dead_code_confidence, dead_code_source).
    dead_code_source: "static" | "static+coverage"
    """
    if is_entry_point or caller_count > 0:
        source = "static+coverage" if coverage.source != "none" else "static"
        return "none", source

    runtime_covered = (
        coverage.source != "none"
        and coverage.is_covered(symbol_path, line_start, line_end)
    )

    if runtime_covered:
        # Runtime execution confirmed — static graph blind spot (dynamic dispatch,
        # reflection, scheduled job, decorator-registered callback).
        return "low", "static+coverage"

    # Static-only path (coverage unavailable or not hit)
    source = "static+coverage" if coverage.source != "none" else "static"
    if is_private:
        return "high", source
    if scope_clear:
        return "medium", source
    return "low", source

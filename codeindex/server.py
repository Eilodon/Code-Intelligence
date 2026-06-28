PRESET_TOOL_SETS: dict[str, set[str] | None] = {
    "orient":   {"repo_overview", "locate", "dependencies", "hotspots", "indexing_status"},
    "trace":    {"repo_overview", "locate", "callers", "callees", "path", "session_context"},
    "edit":     {"repo_overview", "locate", "source", "edit_context", "diff_impact", "indexing_status"},
    "compound": {"repo_overview", "locate", "hotspots", "source", "understand",
                 "edit_context", "diff_impact", "session_context", "indexing_status"},
    "full":     None,  # None = all tools
}

ALL_TOOLS = {
    "repo_overview": lambda: None,
    "search": lambda: None,
    "file_overview": lambda: None,
    "dependencies": lambda: None,
    "symbol_info": lambda: None,
    "source": lambda: None,
    "callers": lambda: None,
    "callees": lambda: None,
    "path": lambda: None,
    "edit_context": lambda: None,
    "session_context": lambda: None,
    "diff_impact": lambda: None,
    "indexing_status": lambda: None,
    "locate": lambda: None,
    "hotspots": lambda: None,
    "understand": lambda: None,
}

class SuggestedNext:
    tool: str
    reason: str
    args: dict

def _raw_suggested_next(tool_name: str, output: dict) -> SuggestedNext | None:
    # Dummy implementation, the document describes conditions
    pass

def compute_suggested_next(
    tool_name: str,
    output: dict,
    available_tools: set[str] | None = None
) -> SuggestedNext | None:
    hint = _raw_suggested_next(tool_name, output)
    if hint is None:
        return None
    if available_tools is not None and hint.tool not in available_tools:
        return None
    return hint

def register_tools(mcp_server, preset: str = "full") -> None:
    if preset not in PRESET_TOOL_SETS:
        valid = list(PRESET_TOOL_SETS.keys())
        raise ValueError(
            f"[codeindex] Unknown preset: {preset!r}. Valid options: {valid}"
        )
    allowed = PRESET_TOOL_SETS[preset]
    for tool_name, tool_fn in ALL_TOOLS.items():
        if allowed is None or tool_name in allowed:
            mcp_server.register_tool(tool_name, tool_fn)

def init_server(project_root):
    import codeindex.codeowners as codeowners
    from codeindex.coverage_reader import CoverageReader
    
    codeowners_patterns = codeowners.load_codeowners(project_root)  # cached once at startup
    if codeowners_patterns:
        print(f"[codeindex] CODEOWNERS loaded: {len(codeowners_patterns)} patterns")
        
    coverage_data = CoverageReader.load(project_root)
    if coverage_data.source != "none":
        print(f"[codeindex] Coverage data loaded: {coverage_data.source}")
    
    return codeowners_patterns, coverage_data

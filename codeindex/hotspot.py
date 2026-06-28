import sqlite3
from pathlib import Path
from typing import Optional, List, Dict, Any

def compute_hotspots(
    project_root: Path,
    conn: sqlite3.Connection,
    top_n: int = 10,
    since: str = "6 months ago",
    min_churn: int = 2,
    include_symbols: bool = False,
    risk_critical_threshold: float = 0.75,
    risk_high_threshold: float = 0.50,
    risk_medium_threshold: float = 0.25,
) -> dict[str, Any]:
    """
    Returns a dict matching the HotspotsOutput TS schema.
    """

    # --- Step 1: Churn from git (optional) ---
    churn_map: dict[str, dict] = {}
    git_available = False
    try:
        import subprocess
        result = subprocess.run(
            ["git", "log", f"--since={since}",
             "--name-only", "--format=|||%ae|||%aI"],  # %aI = strict ISO 8601
            cwd=project_root, capture_output=True, text=True, timeout=30
        )
        if result.returncode == 0:
            git_available = True
            current_author, current_date = None, None
            for line in result.stdout.splitlines():
                if line.startswith("|||"):
                    parts = line.split("|||")
                    current_author = parts[1].strip() if len(parts) > 1 else None
                    current_date = parts[2].strip() if len(parts) > 2 else None
                elif line.strip():
                    abs_path = str(project_root / line.strip())
                    if abs_path not in churn_map:
                        churn_map[abs_path] = {
                            "commit_count": 0,
                            "authors": set(),
                            "last_changed": current_date or None
                        }
                    churn_map[abs_path]["commit_count"] += 1
                    if current_author:
                        churn_map[abs_path]["authors"].add(current_author)
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass

    # --- Step 2: Complexity from index (always available) ---
    rows = conn.execute("""
        SELECT
            path,
            COUNT(*) as symbol_count,
            SUM(CASE WHEN is_hub = 1 THEN 1 ELSE 0 END) as hub_count,
            AVG(COALESCE(caller_count, 0)) as avg_caller_count,
            SUM(CASE WHEN coreness > 0 THEN 1 ELSE 0 END) as connected_coreness_count,
            MAX(language) as language
        FROM symbols
        WHERE path IS NOT NULL
        GROUP BY path
    """).fetchall()
    complexity_map: dict[str, dict] = {}
    for path, sym_count, hub_count, avg_callers, high_core, language in rows:
        complexity_map[path] = {
            "symbol_count": sym_count,
            "hub_count": hub_count or 0,
            "avg_caller_count": round(avg_callers or 0, 2),
            "connected_coreness_count": high_core or 0,
            "language": language,
        }

    # Complexity score per file: weighted combination
    def complexity_score(c: dict) -> float:
        return (
            c["symbol_count"] * 0.3 +
            c["hub_count"] * 3.0 +          # hubs heavily weighted
            c["connected_coreness_count"] * 1.5 +
            c["avg_caller_count"] * 0.5
        )

    # --- Step 3: Merge + normalize ---
    if git_available:
        candidates = {
            path: data for path, data in churn_map.items()
            if data["commit_count"] >= min_churn and path in complexity_map
        }
    else:
        # No git: rank purely by complexity. min_churn not applicable.
        candidates = {path: {"commit_count": 0, "authors": set(), "last_changed": None}
                      for path in complexity_map}

    if not candidates:
        return {
            "hotspots": [],
            "git_available": git_available,
            "since": since,
            "total_files_analyzed": 0,
            "hotspot_method": "git+index" if git_available else "index_only",
            "note": "Git unavailable: ranking by complexity only. min_churn parameter not applied." if not git_available else f"No files exceeded min_churn={min_churn} commits since {since}. Try reducing min_churn."
        }

    total_files_analyzed = len(candidates)

    churn_scores = {p: d["commit_count"] for p, d in candidates.items()}
    compl_scores = {p: complexity_score(complexity_map[p]) for p in candidates}

    max_churn = max(churn_scores.values()) or 1
    max_compl = max(compl_scores.values()) or 1

    results = []
    for path in candidates:
        norm_compl = compl_scores[path] / max_compl
        if git_available:
            norm_churn = churn_scores[path] / max_churn
            score = norm_churn * norm_compl
        else:
            score = norm_compl  # no git: pure complexity score
        
        level = (
            "critical" if score >= risk_critical_threshold else
            "high"     if score >= risk_high_threshold else
            "medium"   if score >= risk_medium_threshold else
            "low"
        )
        cd = candidates[path]
        cm = complexity_map[path]
        
        hotspot_entry = {
            "path": path,
            "language": cm["language"],
            "churn": {
                "commit_count": cd["commit_count"],
                "unique_authors": len(cd.get("authors", set())),
                "last_changed": cd.get("last_changed")
            },
            "complexity": {
                "symbol_count": cm["symbol_count"],
                "hub_count": cm["hub_count"],
                "connected_coreness_count": cm["connected_coreness_count"],
                "avg_caller_count": cm["avg_caller_count"],
            },
            "hotspot_score": round(score, 4),
            "risk_level": level
        }
        results.append(hotspot_entry)

    results.sort(key=lambda r: r["hotspot_score"], reverse=True)
    top_results = results[:top_n]

    if include_symbols:
        for r in top_results:
            symbols_rows = conn.execute("""
                SELECT name, kind, is_hub, coreness, caller_count
                FROM symbols WHERE path = ?
                ORDER BY COALESCE(caller_count, 0) DESC, coreness DESC
                LIMIT 5
            """, (r["path"],)).fetchall()
            
            r["top_symbols"] = [
                {
                    "name": row[0],
                    "kind": row[1],
                    "is_hub": bool(row[2]),
                    "coreness": row[3],
                    "caller_count": row[4]
                }
                for row in symbols_rows
            ]

    note_msg = ""
    if git_available:
        if not top_results:
            note_msg = f"No files exceeded min_churn={min_churn} commits since {since}. Try reducing min_churn."
        else:
            note_msg = f"Analyzed commits since {since}."
    else:
        note_msg = "Git unavailable: ranking by complexity only. min_churn parameter not applied."

    return {
        "hotspots": top_results,
        "git_available": git_available,
        "since": since,
        "total_files_analyzed": total_files_analyzed,
        "hotspot_method": "git+index" if git_available else "index_only",
        "note": note_msg
    }

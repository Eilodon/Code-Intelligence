import os
import sys
import json
from pathlib import Path

# Add project root to sys.path to import codeindex
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent.parent.resolve()))

try:
    from codeindex.codeowners import load_codeowners
    from codeindex.coverage_reader import CoverageReader
except ImportError as e:
    print(f"Failed to import: {e}")
    sys.exit(1)

def generate_oracle():
    fixture_dir = Path(__file__).parent.resolve()
    
    # 1. Codeowners
    owners = load_codeowners(fixture_dir)
    # get_codeowners might return a generator or list of tuples. Let's force list.
    owners_list = list(owners)
    
    with open(fixture_dir / "expected_codeowners.json", "w") as f:
        json.dump(owners_list, f, indent=2)

    # 2. Coverage
    try:
        cov = CoverageReader.load(fixture_dir)
        # Assuming cov has a method or dict structure.
        # Let's inspect cov
        if hasattr(cov, "to_dict"):
            cov_dict = cov.to_dict()
        elif hasattr(cov, "__dict__"):
            cov_dict = cov.__dict__
        else:
            cov_dict = cov
            
        # Convert Sets to Lists and sanitize absolute paths to relative
        if 'covered_lines' in cov_dict:
            sanitized = {}
            for k, v in cov_dict['covered_lines'].items():
                rel_k = k.replace(str(fixture_dir), "{PROJECT_ROOT}")
                sanitized[rel_k] = sorted(list(v))
            cov_dict['covered_lines'] = sanitized

        with open(fixture_dir / "expected_coverage.json", "w") as f:
            json.dump(cov_dict, f, indent=2)
    except Exception as e:
        print(f"Coverage error: {e}")

    # 3. Path, Coreness, Hotspot
    import sqlite3
    db_path = fixture_dir / ".antigravity" / "codeindex.db"
    if db_path.exists():
        conn = sqlite3.connect(db_path)
        
        # Path
        try:
            from codeindex.path_algo import PathFinder
            finder = PathFinder(conn)
            path_result, _, _ = finder.bidirectional_bfs_path("A", "C", 10, 3, 5000)
            routes = []
            for route in path_result:
                steps = []
                for step in route:
                    steps.append({"symbol": step[0], "edge_confidence": step[1]})
                routes.append({"steps": steps, "length": len(route) - 1})
            with open(fixture_dir / "expected_path.json", "w") as f:
                json.dump(routes, f, indent=2)
        except Exception as e:
            import traceback
            traceback.print_exc()
            print(f"Path error: {e}")

        # Coreness
        try:
            from codeindex.db_init import compute_coreness
            coreness_result = compute_coreness(conn)
            with open(fixture_dir / "expected_coreness.json", "w") as f:
                json.dump(coreness_result, f, indent=2)
        except Exception as e:
            print(f"Coreness error: {e}")
            
        # Hotspot
        try:
            from codeindex.hotspot import compute_hotspots
            hotspot_result = compute_hotspots(
                project_root=fixture_dir,
                conn=conn,
                top_n=10,
                since="1 year",
                min_churn=0,
                include_symbols=False
            )
            hotspots = []
            for h in hotspot_result["hotspots"]:
                hotspots.append({
                    "path": h["path"],
                    "hotspot_score": h["hotspot_score"],
                    "churn": h["churn"]["commit_count"],
                    "complexity": h["complexity"]["symbol_count"],
                    "risk_level": h["risk_level"]
                })
            with open(fixture_dir / "expected_hotspot.json", "w") as f:
                json.dump(hotspots, f, indent=2)
        except Exception as e:
            import traceback
            traceback.print_exc()
            print(f"Hotspot error: {e}")
            
        conn.close()

if __name__ == "__main__":
    generate_oracle()

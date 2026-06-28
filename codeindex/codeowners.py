import fnmatch
import subprocess
from pathlib import Path

CODEOWNERS_PATHS = [
    ".github/CODEOWNERS",
    "CODEOWNERS",
    "docs/CODEOWNERS",
    ".gitlab/CODEOWNERS",
]

def load_codeowners(project_root: Path) -> list[tuple[str, list[str]]]:
    """
    Parse CODEOWNERS file. Returns list of (pattern, [owners]).
    GitHub/GitLab: LAST matching rule wins.
    """
    for relative in CODEOWNERS_PATHS:
        path = project_root / relative
        if not path.exists():
            continue
        patterns = []
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split()
            if len(parts) >= 2:
                pattern, *owners = parts
                patterns.append((pattern, owners))
        return patterns
    return []

def find_owners(patterns: list[tuple[str, list[str]]], file_path: str) -> list[str]:
    """
    GitHub CODEOWNERS: last matching rule wins.
    '*' does not cross '/': "src/*.py" matches "src/x.py" but NOT "src/nested/x.py".
    Trailing '/' = explicit directory. No slash = basename match anywhere.
    Slash present (no trailing) = root-anchored, segment-by-segment glob.
    Extension-less files (Makefile, Dockerfile, Procfile) matched by basename, not
    misclassified as directory patterns.
    """
    matched: list[str] = []
    file_path_normalized = file_path.lstrip("/")

    def _match_path_pattern(pattern_parts: list[str], file_parts: list[str]) -> bool:
        """Segment-by-segment match: '*' cannot cross '/'. Segment count must match exactly."""
        if len(pattern_parts) != len(file_parts):
            return False
        return all(fnmatch.fnmatch(fp, pp) for pp, fp in zip(pattern_parts, file_parts))

    for pattern, owners in patterns:
        pattern_normalized = pattern.lstrip("/")

        if pattern_normalized.endswith("/"):
            # Explicit directory: match files under this directory.
            dir_pattern = pattern_normalized[:-1]
            if any(c in dir_pattern for c in "*?["):
                parts = file_path_normalized.split("/")
                matched_dir = False
                for i in range(len(parts)):
                    prefix = "/".join(parts[:i+1])
                    if fnmatch.fnmatch(prefix, dir_pattern):
                        matched_dir = True
                        break
                if matched_dir:
                    matched = owners
            else:
                if file_path_normalized.startswith(pattern_normalized):
                    matched = owners
        elif "/" not in pattern_normalized:
            # No slash → matches any file in any directory by basename
            if fnmatch.fnmatch(Path(file_path_normalized).name, pattern_normalized):
                matched = owners
        else:
            # Path pattern with slash → root-anchored, segment-by-segment.
            pattern_parts = pattern_normalized.split("/")
            file_parts = file_path_normalized.split("/")
            if _match_path_pattern(pattern_parts, file_parts):
                matched = owners

    return matched

def get_git_blame_owners(
    project_root: Path,
    file_path: str,
    top_n: int = 3,
    timeout: float = 5.0,
    since: str = "1 year ago"
) -> list[str]:
    """Fallback: recent committers from git log."""
    try:
        result = subprocess.run(
            ["git", "log", f"--since={since}", "--follow",
             "-n", "10", "--format=%ae", "--", file_path],
            cwd=project_root, capture_output=True, text=True, timeout=timeout
        )
        if result.returncode != 0:
            return []
        authors = []
        seen = set()
        for email in result.stdout.splitlines():
            email = email.strip()
            if email and email not in seen:
                seen.add(email)
                authors.append(email)
                if len(authors) >= top_n:
                    break
        return authors
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return []

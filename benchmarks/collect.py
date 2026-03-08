"""Run ghverify against all benchmark cases and collect raw results."""

import json
import subprocess
import sys
from pathlib import Path

GREEN = "\033[32m"
RED = "\033[31m"
RESET = "\033[0m"


def main():
    if len(sys.argv) < 3:
        print("Usage: collect.py <ghverify_bin> <output.jsonl>")
        sys.exit(1)

    ghverify = sys.argv[1]
    output_path = Path(sys.argv[2])
    cases_dir = Path(__file__).parent / "cases"

    output_path.parent.mkdir(parents=True, exist_ok=True)
    entries = []

    for case_file in sorted(cases_dir.glob("*.json")):
        case = json.loads(case_file.read_text())
        case_id = case["id"]
        repo = case["repo"]
        pr_number = case["pr_number"]
        expected = case["expected"]

        try:
            result = subprocess.run(
                [ghverify, "pr", str(pr_number), "--repo", repo, "--format", "json"],
                capture_output=True, text=True, timeout=30,
            )
            raw_output = json.loads(result.stdout) if result.stdout.strip() else []
            actual = raw_output[0]["severity"] if raw_output else "empty"
        except (json.JSONDecodeError, subprocess.TimeoutExpired, IndexError, KeyError):
            raw_output = []
            actual = "fetch_error"

        if actual == expected:
            print(f"{GREEN}[PASS]{RESET} {case_id:<12} | {repo:<30} #{pr_number:<5} | expected={expected}")
        else:
            print(f"{RED}[FAIL]{RESET} {case_id:<12} | {repo:<30} #{pr_number:<5} | expected={expected:<7} actual={actual}")

        entries.append(json.dumps({
            "id": case_id,
            "expected": expected,
            "actual": actual,
            "case": case,
            "raw_output": raw_output,
        }, ensure_ascii=True))

    output_path.write_text("\n".join(entries) + "\n")
    print(f"\nRaw results: {output_path}")


if __name__ == "__main__":
    main()

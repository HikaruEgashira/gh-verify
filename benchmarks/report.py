"""Compute Accuracy, Precision, Recall, F1 from benchmark raw results."""

import json
import sys
from collections import defaultdict
from pathlib import Path

SEVERITIES = ["pass", "warning", "error"]


def main():
    if len(sys.argv) < 3:
        print("Usage: report.py <raw.jsonl> <output.json>")
        sys.exit(1)

    raw_path = Path(sys.argv[1])
    output_path = Path(sys.argv[2])

    entries = [json.loads(line) for line in raw_path.read_text().splitlines() if line.strip()]

    # Confusion matrix
    tp = defaultdict(int)
    fp = defaultdict(int)
    fn = defaultdict(int)

    results = []
    for e in entries:
        expected = e["expected"]
        actual = e["actual"]
        outcome = "PASS" if actual == expected else "FAIL"
        if actual == expected:
            tp[expected] += 1
        else:
            if actual in SEVERITIES:
                fp[actual] += 1
            fn[expected] += 1
        results.append({
            "id": e["id"],
            "expected": expected,
            "actual": actual,
            "outcome": outcome,
            "case": e["case"],
            "raw_output": e["raw_output"],
        })

    total = len(entries)
    correct = sum(1 for r in results if r["outcome"] == "PASS")

    # Per-severity metrics
    metrics = {}
    for s in SEVERITIES:
        t, f, n = tp[s], fp[s], fn[s]
        precision = t / (t + f) if (t + f) > 0 else None
        recall = t / (t + n) if (t + n) > 0 else None
        if precision is not None and recall is not None and (precision + recall) > 0:
            f1 = 2 * precision * recall / (precision + recall)
        else:
            f1 = None
        metrics[s] = {
            "tp": t, "fp": f, "fn": n,
            "precision": round(precision, 4) if precision is not None else None,
            "recall": round(recall, 4) if recall is not None else None,
            "f1": round(f1, 4) if f1 is not None else None,
        }

    # Macro-averaged F1
    f1_values = [m["f1"] for m in metrics.values() if m["f1"] is not None]
    macro_f1 = sum(f1_values) / len(f1_values) if f1_values else None

    # Print report
    accuracy = correct / total if total > 0 else 0
    print(f"\nAccuracy: {correct}/{total} ({accuracy:.1%})")
    print(f"Macro F1: {macro_f1:.4f}" if macro_f1 else "Macro F1: N/A")
    print()
    print(f"{'Severity':<10} {'TP':>4} {'FP':>4} {'FN':>4} {'Precision':>10} {'Recall':>10} {'F1':>10}")
    print(f"{'--------':<10} {'--':>4} {'--':>4} {'--':>4} {'---------':>10} {'------':>10} {'--':>10}")
    for s in SEVERITIES:
        m = metrics[s]
        p = f"{m['precision']:.1%}" if m["precision"] is not None else "N/A"
        r = f"{m['recall']:.1%}" if m["recall"] is not None else "N/A"
        f = f"{m['f1']:.4f}" if m["f1"] is not None else "N/A"
        print(f"{s:<10} {m['tp']:>4} {m['fp']:>4} {m['fn']:>4} {p:>10} {r:>10} {f:>10}")
    print()

    # Write JSON report
    report = {
        "timestamp": raw_path.stem.replace("raw_", ""),
        "summary": {
            "total": total,
            "correct": correct,
            "accuracy": round(accuracy, 4),
            "macro_f1": round(macro_f1, 4) if macro_f1 else None,
        },
        "metrics": metrics,
        "results": results,
    }
    output_path.write_text(json.dumps(report, indent=2, ensure_ascii=False))
    print(f"Report: {output_path}")


if __name__ == "__main__":
    main()

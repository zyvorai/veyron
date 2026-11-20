#!/usr/bin/env python3
# Copyright 2026 Zyvor AI Labs · https://zyvor.dev
# SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
#
# readiness-report.py — turn the customer-readiness JSONL trail into a signed
# GO / NO-GO report (JSON + Markdown). Prints the verdict word to stdout and
# exits 0 (GO) or 1 (NO-GO).
#
# Verdict rule: GO iff FAIL == 0 AND the P1 acceptance gate passed.
# WARN and SKIP never block; every SKIP carries a machine reason.

import argparse
import json
import sys
from collections import Counter, defaultdict


def load_checks(path):
    checks = []
    try:
        with open(path, encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    checks.append(json.loads(line))
                except json.JSONDecodeError:
                    continue
    except OSError:
        pass
    return checks


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--jsonl", required=True)
    ap.add_argument("--json-out", required=True)
    ap.add_argument("--md-out", required=True)
    ap.add_argument("--host", default="")
    ap.add_argument("--nodes", default="0")
    ap.add_argument("--kubevirt", default="")
    ap.add_argument("--kubevirt-baseline", default="")
    ap.add_argument("--cdi", default="")
    ap.add_argument("--cdi-baseline", default="")
    ap.add_argument("--ceph", default="")
    ap.add_argument("--tls", default="")
    ap.add_argument("--p1-passed", default="0")
    args = ap.parse_args()

    checks = load_checks(args.jsonl)
    tally = Counter(c.get("status", "?") for c in checks)
    n_pass = tally.get("pass", 0)
    n_fail = tally.get("fail", 0)
    n_warn = tally.get("warn", 0)
    n_skip = tally.get("skip", 0)
    p1_passed = args.p1_passed == "1"

    go = (n_fail == 0) and p1_passed
    verdict = "GO" if go else "NO-GO"

    # Group skip reasons into buckets for the summary line.
    skip_reasons = Counter()
    for c in checks:
        if c.get("status") == "skip":
            reason = c.get("reason", "") or "unspecified"
            bucket = reason.split(":", 1)[0]
            skip_reasons[bucket] += 1
    skip_summary = ", ".join(f"{k}:{v}" for k, v in sorted(skip_reasons.items()))

    # Per-phase rollup preserving first-seen order.
    phase_order = []
    per_phase = defaultdict(lambda: Counter())
    for c in checks:
        ph = c.get("phase", "general")
        if ph not in per_phase:
            phase_order.append(ph)
        per_phase[ph][c.get("status", "?")] += 1

    blocking = [c for c in checks if c.get("status") == "fail"]
    warnings = [c for c in checks if c.get("status") == "warn"]
    skipped = [c for c in checks if c.get("status") == "skip"]

    report = {
        "verdict": verdict,
        "cluster": {
            "host": args.host,
            "node_count": _int(args.nodes),
            "kubevirt_version": args.kubevirt,
            "kubevirt_baseline": args.kubevirt_baseline,
            "cdi_version": args.cdi,
            "cdi_baseline": args.cdi_baseline,
            "ceph_health": args.ceph,
            "tls_issuer": args.tls,
        },
        "summary": {
            "passed": n_pass,
            "failed": n_fail,
            "warnings": n_warn,
            "skipped": n_skip,
            "p1_gate_passed": p1_passed,
            "skip_buckets": dict(skip_reasons),
        },
        "phases": [
            {"phase": ph, **dict(per_phase[ph])} for ph in phase_order
        ],
        "checks": checks,
    }
    with open(args.json_out, "w", encoding="utf-8") as fh:
        json.dump(report, fh, indent=2)

    # ── Markdown ────────────────────────────────────────────────────────────
    banner = "✅ **GO**" if go else "⛔ **NO-GO**"
    lines = []
    lines.append(f"# Veyron customer-readiness — {banner}")
    lines.append("")
    lines.append(
        f"**{verdict}** — {n_pass} passed, {n_fail} failed, "
        f"{n_warn} warnings, {n_skip} skipped"
        + (f" ({skip_summary})" if skip_summary else "")
    )
    lines.append("")
    lines.append("## Cluster")
    lines.append("")
    lines.append("| Field | Value |")
    lines.append("|---|---|")
    lines.append(f"| Host | `{args.host}` |")
    lines.append(f"| Nodes | {args.nodes} |")
    lines.append(f"| KubeVirt | {args.kubevirt or '?'} "
                 f"{_drift(args.kubevirt, args.kubevirt_baseline)} |")
    lines.append(f"| CDI | {args.cdi or '?'} "
                 f"{_drift(args.cdi, args.cdi_baseline)} |")
    lines.append(f"| Ceph | {args.ceph or 'unknown'} |")
    lines.append(f"| TLS issuer | {args.tls or 'unknown'} |")
    lines.append(f"| P1 acceptance gate | {'passed' if p1_passed else 'FAILED'} |")
    lines.append("")

    lines.append("## Phases")
    lines.append("")
    lines.append("| Phase | Pass | Fail | Warn | Skip |")
    lines.append("|---|---|---|---|---|")
    for ph in phase_order:
        c = per_phase[ph]
        lines.append(
            f"| {ph} | {c.get('pass', 0)} | {c.get('fail', 0)} "
            f"| {c.get('warn', 0)} | {c.get('skip', 0)} |"
        )
    lines.append("")

    if blocking:
        lines.append("## ⛔ Blocking failures (must fix before go-live)")
        lines.append("")
        for c in blocking:
            detail = f" — {c['detail']}" if c.get("detail") else ""
            lines.append(f"- **[{c.get('phase')}]** {c.get('check')}{detail}")
        lines.append("")

    if warnings:
        lines.append("## ⚠️ Warnings (advisory — do not block, but review)")
        lines.append("")
        for c in warnings:
            detail = f" — {c['detail']}" if c.get("detail") else ""
            lines.append(f"- [{c.get('phase')}] {c.get('check')}{detail}")
        lines.append("")

    if skipped:
        lines.append("## ○ Skipped (not applicable)")
        lines.append("")
        for c in skipped:
            reason = f" _(reason: {c['reason']})_" if c.get("reason") else ""
            lines.append(f"- [{c.get('phase')}] {c.get('check')}{reason}")
        lines.append("")

    lines.append("---")
    lines.append("")
    lines.append(
        "_Verdict rule: **GO** iff zero failures **and** the P1 acceptance gate "
        "passed. Warnings and skips never block. Multi-node checks (live "
        "migration, drain) SKIP on single-node clusters — re-run against a "
        "≥2-node cluster to exercise them._"
    )
    lines.append("")

    with open(args.md_out, "w", encoding="utf-8") as fh:
        fh.write("\n".join(lines))

    print(verdict)
    sys.exit(0 if go else 1)


def _int(s):
    try:
        return int(s)
    except (TypeError, ValueError):
        return 0


def _drift(got, baseline):
    if not baseline or not got:
        return ""
    return "✓" if got == baseline else f"⚠️ drift (baseline {baseline})"


if __name__ == "__main__":
    main()

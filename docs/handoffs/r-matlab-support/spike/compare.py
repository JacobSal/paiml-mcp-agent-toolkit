"""Diff tree-sitter definitions against the mtree oracle and evaluate K1.

Usage: python compare.py ORACLE_CSV TS_CSV
"""
import csv
import sys
from collections import Counter


def load(path):
    with open(path, newline="", encoding="utf-8") as fh:
        return list(csv.DictReader(fh))


oracle, ts = load(sys.argv[1]), load(sys.argv[2])
bad_files = {r["file"] for r in oracle if r["kind"] == "parse_error"}
defs = lambda rows: Counter(
    (r["file"], r["name"], r["line"]) for r in rows
    if r["kind"] in ("function", "classdef") and r["file"] not in bad_files
)
o, t = defs(oracle), defs(ts)
n_oracle = sum(v for k, v in o.items() if k[1:] and True)
n_fn = sum(v for k, v in o.items())
exact = sum((o & t).values())
miss, extra = o - t, t - o

ts_fn = [r for r in ts if r["kind"] in ("function", "classdef") and r["file"] not in bad_files]
span_err = sum(int(r["span_error"]) for r in ts_fn)
file_rows = [r for r in ts if r["kind"] == "file_error"]
file_err = sum(int(r["span_error"]) for r in file_rows)

print(f"oracle defs (parseable files): {n_fn}   tree-sitter defs: {sum(t.values())}")
print(f"exact name+line matches: {exact}  ({100 * exact / n_fn:.2f}%)")
print(f"oracle-only (missed): {sum(miss.values())}   tree-sitter-only (extra): {sum(extra.values())}")
print(f"span-level ERROR/MISSING: {span_err}/{len(ts_fn)} ({100 * span_err / len(ts_fn):.2f}%)")
print(f"file-level any-ERROR (v1 K1): {file_err}/{len(file_rows)} ({100 * file_err / len(file_rows):.2f}%)")
mism = 100 * (n_fn - exact) / n_fn
k1 = mism >= 5 or 100 * span_err / len(ts_fn) >= 5
print(f"K1 (v2): mismatch {mism:.2f}% / span-error {100 * span_err / len(ts_fn):.2f}% -> {'KILL' if k1 else 'GO'}")
for label, c in (("MISSED", miss), ("EXTRA", extra)):
    for k in sorted(c):
        print(f"  {label}: {k[0]}:{k[2]} {k[1]}")
for r in ts_fn:
    if r["span_error"] == "1":
        print(f"  SPAN_ERR: {r['file']}:{r['line']} {r['name']}")
print("mtree parse_error files (excluded):")
for f in sorted(bad_files):
    got = [r for r in ts if r["file"] == f and r["kind"] == "function"]
    fe = next(r["span_error"] for r in ts if r["file"] == f and r["kind"] == "file_error")
    print(f"  {f}: tree-sitter found {len(got)} defs, file has_error={fe}")

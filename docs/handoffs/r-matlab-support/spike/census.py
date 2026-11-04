"""Census of a repo's MATLAB/R files: scripts vs function files vs classdef."""
import re
import subprocess
import sys

repo = sys.argv[1]
files = subprocess.run(["git", "-C", repo, "ls-files", "*.m", "*.R", "*.r"],
                       capture_output=True, text=True).stdout.splitlines()

m = {"function": 0, "script": 0, "classdef": 0}
m_defs = m_cells = m_cmdsyntax = 0
r_files = r_withfn = r_defs = 0
fn_re = re.compile(r"^\s*function\b", re.M)
r_fn_re = re.compile(r"(<-|<<-|=)\s*function\s*\(")
cmd_re = re.compile(r"^\s*(hold|clear|close|format|warning|addpath|figure)\s+\w", re.M)

for f in files:
    try:
        src = open(f"{repo}/{f}", encoding="utf-8", errors="replace").read()
    except OSError:
        continue
    if f.endswith(".m"):
        code = [l for l in src.splitlines() if l.strip() and not l.strip().startswith("%")]
        first = code[0] if code else ""
        kind = "classdef" if first.lstrip().startswith("classdef") else \
               "function" if first.lstrip().startswith("function") else "script"
        m[kind] += 1
        m_defs += len(fn_re.findall(src))
        m_cells += len(re.findall(r"^\s*%%", src, re.M))
        m_cmdsyntax += len(cmd_re.findall(src))
    else:
        r_files += 1
        n = len(r_fn_re.findall(src))
        r_defs += n
        r_withfn += n > 0

print(f"MATLAB files: {m}  function-defs={m_defs}  %%-cells={m_cells}  command-syntax-lines={m_cmdsyntax}")
print(f"R files: {r_files}  with-fn-defs={r_withfn}  fn-defs={r_defs}")

"""Extract MATLAB definitions with tree-sitter-matlab + its tags.scm.

Usage: python ts_extract.py REPO LIST_FILE OUT_CSV

Writes file,name,line,kind,span_error rows (kind = function | classdef |
file_error). span_error is 1 when the definition's subtree contains an
ERROR or MISSING node. One file_error row per file records whether the
file has any error at all, so per-file and per-span rates can be compared.
"""
import csv
import pathlib
import sys

import tree_sitter_matlab as tsm
from tree_sitter import Language, Parser, Query, QueryCursor

LANG = Language(tsm.language())
TAGS = (pathlib.Path(tsm.__file__).parent / "queries" / "neovim" / "tags.scm").read_text()
QUERY = Query(LANG, TAGS)
PARSER = Parser(LANG)


def def_name(def_node, name_node):
    """Name of a definition; property accessors (get.X / set.X) are rebuilt."""
    name = name_node.text.decode()
    prev = name_node.prev_sibling
    # Accessors carry a single `get.` / `set.` token before the identifier.
    if prev is not None and prev.type in ("get.", "set."):
        name = prev.type + name
    return name


def main(repo, list_file, out_csv):
    files = [l.strip() for l in open(list_file, encoding="utf-8") if l.strip()]
    with open(out_csv, "w", newline="", encoding="utf-8") as fh:
        w = csv.writer(fh)
        w.writerow(["file", "name", "line", "kind", "span_error"])
        for rel in files:
            src = pathlib.Path(repo, rel).read_bytes()
            tree = PARSER.parse(src)
            w.writerow([rel, "", "", "file_error", int(tree.root_node.has_error)])
            for _, caps in QueryCursor(QUERY).matches(tree.root_node):
                for kind in ("definition.function", "definition.class"):
                    if kind in caps:
                        node, name = caps[kind][0], caps["name"][0]
                        w.writerow([
                            rel,
                            def_name(node, name),
                            node.start_point[0] + 1,
                            "function" if kind.endswith("function") else "classdef",
                            int(node.has_error),
                        ])


if __name__ == "__main__":
    main(*sys.argv[1:4])

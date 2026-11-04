# R oracle: every function bound by <-, <<-, = (statement form) or ->,
# at any depth, with the exact line R's own parser (getParseData) reports.
# Named call arguments such as tryCatch(error = function(e) ...) are not
# bindings and are excluded; R parses those as EQ_SUB, not EQ_ASSIGN.
# Usage: Rscript r_oracle.R REPO LIST_FILE OUT_CSV
args <- commandArgs(trailingOnly = TRUE)
repo <- args[[1]]
files <- readLines(args[[2]])
files <- files[nzchar(files)]

ASSIGN_LEFT <- c("LEFT_ASSIGN", "EQ_ASSIGN")

defs_in <- function(pd) {
  out <- list()
  fn_exprs <- pd$parent[pd$token == "FUNCTION"]
  for (fe in fn_exprs) {
    asg <- pd$parent[pd$id == fe]
    kids <- pd[pd$parent == asg, ]
    kids <- kids[order(kids$line1, kids$col1), ]
    op <- which(kids$token %in% c(ASSIGN_LEFT, "RIGHT_ASSIGN"))
    if (length(op) != 1) next
    target_id <- if (kids$token[op] == "RIGHT_ASSIGN") kids$id[op + 1] else kids$id[op - 1]
    fn_side <- if (kids$token[op] == "RIGHT_ASSIGN") kids$id[op - 1] else kids$id[op + 1]
    if (fn_side != fe) next
    leaf <- pd[pd$parent == target_id & pd$token %in% c("SYMBOL", "STR_CONST"), ]
    if (nrow(leaf) != 1) next
    name <- gsub("^[`'\"]|[`'\"]$", "", leaf$text)
    out[[length(out) + 1]] <- c(name, pd$line1[pd$id == asg])
  }
  out
}

rows <- list()
for (rel in files) {
  exprs <- tryCatch(parse(file.path(repo, rel), keep.source = TRUE),
                    error = function(err) err)
  if (inherits(exprs, "error")) {
    rows[[length(rows) + 1]] <- data.frame(file = rel, name = "", line = NA, kind = "parse_error")
    next
  }
  found <- defs_in(getParseData(exprs, includeText = TRUE))
  if (length(found)) {
    m <- do.call(rbind, found)
    rows[[length(rows) + 1]] <- data.frame(file = rel, name = m[, 1], line = as.integer(m[, 2]),
                                           kind = "function")
  }
}
write.csv(do.call(rbind, rows), args[[3]], row.names = FALSE, quote = FALSE)

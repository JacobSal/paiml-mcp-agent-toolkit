function mtree_oracle(repo, list_file, out_csv)
%MTREE_ORACLE Write every function definition MATLAB's own parser sees.
%   MTREE_ORACLE(REPO, LIST_FILE, OUT_CSV) parses each repo-relative .m path
%   in LIST_FILE with mtree and writes file,name,line,kind rows to OUT_CSV.
%   kind is "function" for each FUNCTION node, "classdef" for a class, and
%   "parse_error" (line = error line) when mtree cannot parse the file.
files = strtrim(splitlines(fileread(list_file)));
files = files(files ~= "");
fid = fopen(out_csv, 'w');
cleanup = onCleanup(@() fclose(fid));
fprintf(fid, 'file,name,line,kind\n');
for i = 1:numel(files)
    rel = files{i};
    t = mtree(fullfile(repo, rel), '-file');
    if count(t) == 1 && iskind(t, 'ERR')
        msg = string(t.string);
        ln = regexp(msg, '\d+', 'match', 'once');
        fprintf(fid, '%s,%s,%s,parse_error\n', rel, '', ln);
        continue
    end
    cls = mtfind(t, 'Kind', 'CLASSDEF');
    for k = indices(cls)
        c = select(t, k);
        fprintf(fid, '%s,%s,%d,classdef\n', rel, class_name(c), lineno(c));
    end
    fns = mtfind(t, 'Kind', 'FUNCTION');
    for k = indices(fns)
        f = select(t, k);
        fprintf(fid, '%s,%s,%d,function\n', rel, func_name(f), lineno(f));
    end
end
end

function name = func_name(f)
% Fname is an ID, or a DOT for property accessors such as get.Prop.
n = Fname(f);
name = strjoin(strings(Full(n)), '');
end

function name = class_name(c)
% The class name is the left-most ID under the Cexpr (handles "< handle").
ids = mtfind(Full(Cexpr(c)), 'Kind', 'ID');
s = strings(ids);
name = s{1};
end

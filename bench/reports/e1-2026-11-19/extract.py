import json, subprocess, sys, tarfile, pathlib, random
S = pathlib.Path(sys.argv[1]); bench = pathlib.Path(sys.argv[2])
rows = [json.loads(l) for l in open(bench / "bench/results/calibration.jsonl")]
out = []
for r in rows:
    if not r["tauten"].get("errors"): continue
    tb = pathlib.Path(r["work"])
    if not tb.name.endswith(".tar.gz"):
        name = tb.name; dest = bench / "bench/work" / tb.parent.name
    else:
        name = tb.name[:-7]; dest = S / "e1/repos" / tb.parent.name
    if not (dest / name).exists():
        dest.mkdir(parents=True, exist_ok=True)
        with tarfile.open(tb) as t: t.extractall(dest, filter="data")
    repo = dest / name / "repo"
    import tomllib
    host = tomllib.load(open(bench / "bench/tasks" / r["task"] / "task.toml", "rb"))["host"]
    tiers = tomllib.load(open(bench / "bench/hosts" / host / "bench.toml", "rb"))["tiers"]
    args = [str(S / "ct002/bin/cargo-tauten"), "tauten", "check", "--format", "json", "--no-baseline"]
    for c, tr in tiers.items(): args += ["--assume-tier", f"{c}={tr}"]
    p = subprocess.run(args,
                       cwd=repo, capture_output=True, text=True)
    fs = []
    for line in p.stdout.splitlines():
        try: d = json.loads(line)
        except json.JSONDecodeError: continue
        if isinstance(d, dict) and d.get("severity") == "error" and d.get("id") != "TAU-D001": fs.append(d)
    label = r["label"].split(" date")[0]
    if len(fs) != r["tauten"]["errors"]:
        print("MISMATCH", tb, len(fs), r["tauten"]["errors"], file=sys.stderr)
    for f in fs:
        out.append({"label": label, "task": r["task"], "trial": r["trial"], "repo": str(repo),
                    "escaped": bool(r["escaped_defect"]), **f})
json.dump(out, open(S / "e1/findings.json", "w"), indent=1)
print(len(out)); print(json.dumps(out[0])[:600])

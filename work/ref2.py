import re, glob, collections
pat = re.compile(r"\x1b\[[0-9;]*m")
files = collections.Counter()
sample = {}
for p in sorted(glob.glob("*.err.log")):
    with open(p, encoding="utf-8", errors="replace") as f:
        for l in f:
            l = pat.sub("", l).rstrip("\n")
            i = l.find("refused [")
            if i < 0: continue
            files[p] += 1
            sample.setdefault(p, l[i:i+700])
for p, n in files.most_common(6):
    print(f"\n=== {n} in {p}")
    print(sample[p])

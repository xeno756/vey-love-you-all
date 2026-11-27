import re, glob, collections
pat = re.compile(r"\x1b\[[0-9;]*m")
why = collections.Counter()
refused = collections.Counter()
files = sorted(glob.glob("*diagnostic*.err.log"))
print("diagnostic logs:", len(files))
for p in files:
    with open(p, encoding="utf-8", errors="replace") as f:
        for l in f:
            l = pat.sub("", l).rstrip("\n")
            m = re.search(r"dropped ([\d.]+)/frame without a host pipeline: (.+)$", l)
            if m:
                why[m.group(2).strip()] += float(m.group(1))
            m2 = re.search(r"refused \[([^;]+)\]", l)
            if m2:
                refused[m2.group(1).strip()] += 1
print("\n--- dropped/frame by reason (summed over frames) ---")
for s, n in why.most_common(25):
    print(f"{n:10.0f}  {s}")
print("\n--- refusal lines (count) ---")
for s, n in refused.most_common(15):
    print(f"{n:6}  {s[:120]}")

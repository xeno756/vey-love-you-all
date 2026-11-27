import re, glob, collections
pat = re.compile(r"\x1b\[[0-9;]*m")
forms = collections.Counter()
for p in glob.glob("*.err.log"):
    with open(p, encoding="utf-8", errors="replace") as f:
        for l in f:
            l = pat.sub("", l)
            if "without a host pipeline" in l or "no host pipeline" in l:
                s = re.sub(r"\d+", "N", l.split(": ", 2)[-1])[:150]
                forms[s] += 1
for s, n in forms.most_common(20):
    print(f"{n:6}  {s}")

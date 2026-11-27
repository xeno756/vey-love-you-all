import re, glob
pat = re.compile(r"\x1b\[[0-9;]*m")
p = sorted(glob.glob("*diagnostic*.err.log"))[-1]
print("file:", p)
seen=set()
with open(p, encoding="utf-8", errors="replace") as f:
    for l in f:
        l = pat.sub("", l).rstrip("\n")
        if "refused [" in l or "no host pipeline" in l:
            key = l.split("refused [")[0][-40:] + l.split("refused [")[-1][:60]
            if key in seen: continue
            seen.add(key)
            print(l[l.find("refused"):][:400] if "refused" in l else l[:400])

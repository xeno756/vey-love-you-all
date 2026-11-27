import re
pat = re.compile(r"\x1b\[[0-9;]*m")
p = "20261125-144431-series-x-diagnostic-scarlettport-rapide.err.log"
lines=[]
with open(p, encoding="utf-8", errors="replace") as f:
    for l in f:
        lines.append(pat.sub("", l).rstrip("\n"))
for i,l in enumerate(lines):
    if "refused [" in l:
        print("\n".join(lines[max(0,i-2):i+4]))
        break

import re
pat = re.compile(r"\x1b\[[0-9;]*m")
p = "20261127-040848-series-x-normal-scarlettport-performance-vitesse2-qualite-s-relais-fenetres-soleil1-attentes-textures-personnages.err.log"
lines=[]
with open(p, encoding="utf-8", errors="replace") as f:
    for l in f: lines.append(pat.sub("", l).rstrip())
# find the last "ms/frame" and print the block around it
idx=[i for i,l in enumerate(lines) if "ms/frame" in l]
i=idx[-1]
for l in lines[max(0,i-22):i+1]:
    print(l[:600])

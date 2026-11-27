import re, glob
pat = re.compile(r"\x1b\[[0-9;]*m")
rx = re.compile(r"([\d.]+) fps, ([\d.]+) ms/frame .*?(\d+) draws/frame, dropped (\d+) without a host pipeline \((\d+) being made\) and (\d+) indirect")
for p in sorted(glob.glob("*normal*.err.log")):
    best=None; tot=0; n=0; drops=0
    with open(p, encoding="utf-8", errors="replace") as f:
        for l in f:
            l=pat.sub("",l)
            m=rx.search(l)
            if not m: continue
            d=int(m.group(3)); n+=1
            if d>200: drops+=int(m.group(4))
            k=(d,int(m.group(4)))
            if best is None or k[0]>best[0][0]: best=(k,m)
    if best:
        print(f"{p[:70]:72} frames={n:5} maxdraws={best[0][0]:5} drop@{best[0][0]}={best[0][1]:5} fps={best[1].group(1)}")

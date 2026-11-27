import re, glob, statistics
pat = re.compile(r"\x1b\[[0-9;]*m")
p = "20261127-040848-series-x-normal-scarlettport-performance-vitesse2-qualite-s-relais-fenetres-soleil1-attentes-textures-personnages.err.log"
rx = re.compile(r"([\d.]+) fps, ([\d.]+) ms/frame \(([\d.]+) preparing: copies ([\d.]+) ms ([\d.]+) MB, textures ([\d.]+) ms ([\d.]+) MB; ([\d.]+) on the GPU; ([\d.]+) copying back [\d.]+ MB\), (\d+) draws/frame")
hx = re.compile(r"heap buffers ([\d.]+) passes ([\d.]+) ms \(([\d.]+) / ([\d.]+) / ([\d.]+) ms: slots made / copies brought up to date / textures\)")
rows=[]
with open(p, encoding="utf-8", errors="replace") as f:
    for l in f:
        l=pat.sub("",l)
        m=rx.search(l); h=hx.search(l)
        if m and h:
            rows.append([float(x) for x in m.groups()[:9]]+[int(m.group(10))]+[float(x) for x in h.groups()])
if rows:
    cols = ["fps","ms","prep","cop_ms","cop_mb","tex_ms","tex_mb","gpu","back","draws","heap_ms","slots_ms","copup_ms","htex_ms"]
    print("frames:", len(rows))
    for i,c in enumerate(cols):
        v=[r[i] for r in rows]
        print(f"{c:9} median {statistics.median(v):9.1f}   p90 {sorted(v)[int(len(v)*0.9)]:9.1f}   max {max(v):9.1f}")

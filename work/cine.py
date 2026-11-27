import re, glob, statistics
pat = re.compile(r"\x1b\[[0-9;]*m")
p = glob.glob("20261127-031005-*.err.log")[0]
rx = re.compile(r"T(\d\d):(\d\d):([\d.]+)Z.*?([\d.]+) fps, ([\d.]+) ms/frame \(([\d.]+) preparing: copies ([\d.]+) ms ([\d.]+) MB, textures ([\d.]+) ms ([\d.]+) MB; ([\d.]+) on the GPU; ([\d.]+) copying back [\d.]+ MB\), (\d+) draws/frame, dropped (\d+)")
rows=[]
for l in open(p, encoding="utf-8", errors="replace"):
    l=pat.sub("",l); m=rx.search(l)
    if m and int(m.group(1))==8:
        rows.append([float(m.group(i)) for i in range(4,15)])
print("frames 08:xx:", len(rows))
for i,c in enumerate(["fps","ms","prep","cop_ms","cop_mb","tex_ms","tex_mb","gpu","back","draws","dropped"]):
    v=[r[i] for r in rows]
    print(f"{c:8} median {statistics.median(v):8.1f} max {max(v):8.1f}")

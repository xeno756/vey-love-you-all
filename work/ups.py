import re, glob
pat = re.compile(r"\x1b\[[0-9;]*m")
p = "20261127-040848-series-x-normal-scarlettport-performance-vitesse2-qualite-s-relais-fenetres-soleil1-attentes-textures-personnages.err.log"
with open(p, encoding="utf-8", errors="replace") as f:
    for l in f:
        l = pat.sub("", l).rstrip()
        if re.search(r"upscal|render scale|resolution lever|patched|series-s|quality", l, re.I):
            print(l[:300])

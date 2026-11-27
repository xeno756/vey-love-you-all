import re
pat = re.compile(r"\x1b\[[0-9;]*m")
p = "20261127-040848-series-x-normal-scarlettport-performance-vitesse2-qualite-s-relais-fenetres-soleil1-attentes-textures-personnages.err.log"
lines=[pat.sub("",l).rstrip() for l in open(p,encoding="utf-8",errors="replace")]
sel=[l for l in lines if "pages spread over threads" in l or "waiting for a texture another thread" in l or "staging pool since the start" in l]
for l in sel[-6:]:
    print(l[l.find("ms", l.find("INFO")):][:900] if False else l[-900:])
    print("---")

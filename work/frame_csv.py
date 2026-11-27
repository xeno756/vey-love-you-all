import csv, statistics
p = r"C:\Users\fatal\Downloads\Nouveau dossier\victory-logs\20261127-040848-series-x-normal-scarlettport-performance-vitesse2-qualite-s-relais-fenetres-soleil1-attentes-textures-personnages.images.csv"
rows=list(csv.DictReader(open(p, encoding="utf-8-sig", errors="replace")))
def f(r,k):
    try: return float(r[k] or 0)
    except: return 0.0
world=[r for r in rows if f(r,"draws")>1000]
print("rows", len(rows), "world", len(world))
for k in ["frame_ms","cpu_ms","gpu_ms","back_ms","draws","dropped","tex_waiting","pool_held_mb","card_mb","card_budget_mb","shared_mb","page_faults"]:
    v=sorted(f(r,k) for r in world)
    if not v: continue
    print(f"{k:16} median {statistics.median(v):9.1f}  p95 {v[int(len(v)*.95)]:9.1f}  max {v[-1]:9.1f}")
busy=[100*f(r,"gpu_ms")/max(1,f(r,"frame_ms")) for r in world if f(r,"gpu_ms")>0]
print("frames with gpu timing:", len(busy), "median busy %", round(statistics.median(busy),1) if busy else None)
cpu=[100*f(r,"cpu_ms")/max(1,f(r,"frame_ms")) for r in world]
print("median cpu share %", round(statistics.median(cpu),1))

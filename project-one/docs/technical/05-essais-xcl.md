# Essais de XCL sur cette machine (24/11/2026)

Gardés pour mémoire : XCL n'est pas la base de PROJECT ONE, mais ces essais ont fixé
plusieurs décisions. Copie de XCL : `xcl-dev/xcl` au commit `9e3831e`. Les deux corrections
ci-dessous restent dans une copie de travail de XCL, sans commit. Aucune n'est dans PROJECT
ONE.

| Essai | Résultat | Cause |
|---|---|---|
| Series X, DXC 1.6 du SDK | crash à 259 s ; shaders « invalid DXIL container » | DXC trop ancien ; puis réinitialisation de la carte (`0x887A0007`) au chargement du monde (86 826 dessins traduits) |
| DXC 1.9.2609 installé | plus de shaders invalides | — |
| Series S | crash à 41 s | texture R9G9B9E5 render target + UAV refusée, erreur renvoyée au jeu → corrigé comme la PR #605 |
| Series S + correction | blocage à 34 s, un cœur occupé | interblocage : le verrou de mémoire de XCL est tenu pendant un appel D3D12, et D3D12 rappelle XCL qui attend ce verrou → PR #1478 appliquée |
| Series S + 2 corrections | 30 fps stables pendant 3 min, le monde ne se charge jamais | inconnu (aucune erreur) |
| Series X + 2 corrections | crash à 80 s | mémoire vidéo épuisée (`CreateHeap` de 64 Mo → `E_OUTOFMEMORY`) |

## Constats retenus

- **XCL ne dessine la géométrie à mesh shaders de PROJECT GAME sur aucune carte** : la règle
  qui refuse ces pipelines s'applique partout, et le chemin de dessin manque (#724, #529).
- **Le jeu envoie un rapport de crash** de 25 Mo au serveur de son éditeur si le réseau est
  ouvert. XCL le bloque avec `XCL_NETWORK=0` ; VIctory est hors ligne par conception.
- **Licence** : XCL est GPL-2.0-only, donc incompatible avec PROJECT ONE.

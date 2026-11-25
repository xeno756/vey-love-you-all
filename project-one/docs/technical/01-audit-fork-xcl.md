# Agent 1 — Audit : fork de VIctory (fxceless) face à XCL

Audit en lecture seule, le 24/11/2026. Fork au commit `7b0f736`, XCL au commit `9e3831e`
(`xcl-dev/xcl`, main). Les numéros de ligne sont ceux de ces commits. **F** désigne
`crates/` du fork, **X** désigne `xcl/crates/xcl/src/`.

## Matrice de décision

| Module | Meilleur ici | Preuves | Idée de XCL à réécrire |
|---|---|---|---|
| Loader / PE | fork | F `docs/architecture.md:48-62` (le fork lie lui-même les imports) ; XCL modifie les fonctions de fichiers de Windows (X `vfs_hooks.rs:1-8`). Le fork atteint le prologue, XCL jamais le monde ici. | aucune |
| Noyau / services | fork | autonome et hors ligne ; XCL renvoie à `xgameruntime.dll` du système ce qu'il ne fait pas (X `hle/xgameruntime/api.rs:13-16`) | statistiques mémoire : XCL plafonne « utilisé » à la limite (X `kernelx.rs:678-689`) ; le fork rapporte tout l'engagement du processus (F `victory-hle/.../xmem.rs:866-879`) |
| Fichiers / DirectStorage / streaming | fork | un seul groupe de fils pour toute la machine, tampon de 32 Mio (F `dstorage.rs:7-16,334,423`) ; XCL : un fil par file | — |
| Modèle mémoire | fork pour la mémoire vidéo ; aucun des deux ne tient en 16 Go | fork : copies plafonnées à 512 Mo et 80 % du budget (F `victory-gpu/src/shadow.rs:145-155,205-214`) ; XCL : toute la mémoire graphique du jeu en mémoire vidéo, blocs de 64 Mio sans budget (X `hle/d3d12x/vram.rs:162,284-306`), d'où le `CreateHeap 64 MB → E_OUTOFMEMORY` mesuré | render targets qui partagent des pages de 64 Kio, libérés après 2 s d'inactivité (X `vram.rs:1-24,394-450`), avec un plafond en plus |
| D3D12.X | fork | descripteurs sondés et plafonnés (F `descriptor/host.rs:509-590`) ; XCL : interblocage du verrou mémoire (#1478) | chien de garde de 5 s sur les attentes (X `list_x.rs:57-63`) ; R9G9B9E5 (fait, autrement) |
| Shaders | fork | DXIL écrit directement, DXC seulement pour les tests (F `victory-shader/src/emit/validator.rs:1-6`) ; XCL : DXC du SDK, 0,1 à 0,7 Go par compilation | limite de boucles à 2^16 (X `gpu/recomp/mod.rs:622-627`) ; compilations admises selon la mémoire engagée libre (X `dxc.rs:383-445`) |
| Textures | mitigé | le fork saute les textures en tiling XOR (`_X`) (F `victory-gpu/src/texture.rs:1000-1003`), XCL les gère (X `gpu/texture.rs:17-31`) ; BCPack : même code (XCL a repris celui du fork) | adressage `_X`, à refaire depuis addrlib (MIT), pas depuis XCL |
| Diagnostics | fork, avec des manques en version normale | DRED seulement en version de diagnostic (F `host.rs:1205-1211`) ; XCL : DRED en version normale et vérification au démarrage (X `preflight.rs:130-160`) | DRED en version normale ; vérification au démarrage |

## Les 5 idées de XCL classées pour cette machine

| # | Idée | Effort estimé |
|---|---|---|
| 1 | Remplacement des mesh shaders : XCL passe déjà par un calcul qui écrit les sommets (X `gpu/recomp/pso.rs:559-640`) ; le fork les dessinerait avec un vertex shader | 3 à 6 semaines |
| 2 | Render targets qui partagent les pages, avec plafond | 1 à 2 semaines |
| 3 | R9G9B9E5 → R11G11B10 | moins d'un jour (**fait**, agent 3) |
| 4 | Limite de boucles plus basse ou adaptative | 1 à 2 jours |
| 5 | Compilations admises selon la mémoire libre, statistiques mémoire honnêtes | 2 à 4 jours |

## Licence

Code de XCL trouvé dans le fork (`victory-loader/src/console_insn.rs`, `console_insn_decode.rs`,
37 longues lignes identiques à X `insn.rs`) : **retiré** (voir `COMPATIBILITY_REPORT.md`).

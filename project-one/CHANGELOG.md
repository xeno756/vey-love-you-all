# Changelog de PROJECT ONE

Branche `project-one/integration` du fork de VIctory par fxceless. Étiquettes :
- `project-one/baseline` : le fork tel que cloné (`7b0f736`) ;
- `project-one/upstream-a803c87` : le fork à jour sur lequel PROJECT ONE est rebasé.

## 24/11/2026

- **Shaders : les programmes mesh en calcul** (`crates/victory-shader/src/emit/mesh_compute.rs`).
  Le compilateur traduit un programme mesh de la console en compute shader. Celui-ci écrit,
  pour chaque groupe, ses sommets puis ses triangles dans un tampon d'enregistrements. Un
  vertex shader généré relit ces enregistrements et les dessine avec le pixel shader du jeu,
  inchangé. Les triangles éliminés deviennent un point hors de l'écran.
- **GPU : branchement du remplacement des mesh shaders.** Sur une carte sans mesh shaders
  comme la RX 5600 XT (ou partout avec `VICTORY_MESH_COMPUTE=1`), chaque pipeline mesh du jeu
  devient deux pipelines : un de calcul et un de dessin. Chaque `DispatchMesh` devient une
  suite de tranches. Une tranche contient autant de groupes que le tampon de la liste en tient
  (8 Mio par défaut, réglable par `VICTORY_MESH_RECORDS_MB`). Pour chaque tranche : calcul,
  barrière, `DrawInstanced`, puis barrière. `VICTORY_MESH_COMPUTE=0` remet le refus d'avant.
  Les dessins mesh indirects (`ExecuteIndirect`) restent sautés pour l'instant.
- **Loader : retrait du code repris de XCL.** Les fichiers `console_insn.rs` et
  `console_insn_decode.rs` venaient de XCL, sous licence GPL-2.0-only, incompatible avec
  GPL-3.0. Le gestionnaire d'origine de VIctory couvre les mêmes instructions, et ce
  processeur les exécute nativement.
- **GPU : R9G9B9E5 en render target + UAV.** Cette texture est remplacée par une
  R11G11B10_FLOAT de même taille quand la carte la refuse, comme la RX 5600 XT le fait en mode
  Series S. Les relectures de textures remplacées sont sautées.
- **GPU : rapport de perte de la carte.** Il indique la durée du travail, si elle atteint les
  2 s de Windows, et les niveaux de préemption de la carte.
- **GPU : mémoire de transfert des listes.** Au-delà de 256 Mio, elle n'est créée que sous
  90 % du budget de mémoire partagée. Sinon, on libère la réserve, puis on attend qu'une liste
  rende de la mémoire, 2 s au maximum, au lieu d'échouer.
- **GPU : plancher des textures.** Il vaut 2 Gio ou un quart du budget de la carte, le plus
  petit des deux.
- **Démarrage :** une ligne `machine:` indique les processeurs, la RAM, les budgets et les
  limites choisies.
- **Documentation :** dossier `project-one/` (ce fichier et ses voisins, et `docs/technical/`).

Base : fork `a803c87`. Il apporte notamment la correction du test des mesh shaders sur les
cartes qui n'en ont pas, l'optimisation des shaders (environ 43 % d'instructions en moins) et
la limite du tas de vues pour les cartes AMD.

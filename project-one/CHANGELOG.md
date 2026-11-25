# Changelog de PROJECT ONE

Branche `project-one/integration` du fork de VIctory par fxceless. Étiquettes :
- `project-one/baseline` : le fork tel que cloné (`7b0f736`) ;
- `project-one/upstream-a803c87` : le fork à jour sur lequel PROJECT ONE est rebasé.

## 25/11/2026 (suite) — définition réelle, introduction, ScarlettPort

Détail : [docs/technical/08-resolution-scarlettport-intro.md](docs/technical/08-resolution-scarlettport-intro.md).
Reprise du travail : [REPRISE.md](REPRISE.md). Rien de cette section n'a encore tourné avec
PROJECT GAME.

- **Définition réelle.** `lancer.cmd series-x 1080p` : le jeu lui-même calcule son image et son
  rendu 3D en 1920 × 1080 (`720p` et `540p` aussi). `sortie:WxH` règle la taille de l'image du
  jeu, `echelle:N` la part de cette image à laquelle la 3D est calculée. Le journal donne la
  taille demandée par le jeu, celle mise à la place, celle du swap chain, et le nombre de dessins
  par taille de viewport.
- **Hooks de ScarlettPort** repris pour cette version du jeu (GPL-3.0-or-later) : échelle de
  rendu, upscaler du jeu, chien de garde. Le chien de garde du jeu (fermeture après 30 secondes
  sans avancer) est désarmé par défaut ; `chien-de-garde` le remet.
- **Cinématique d'introduction.** Les images sont présentées calées sur l'écran (deux
  rafraîchissements par image à 60 Hz pour 30 images par seconde) au lieu de partir au premier
  rafraîchissement venu. `sans-synchro` remet l'ancien comportement.
- **Mesures vidéo.** Lecteurs vidéo ouverts, images montrées, images sautées pour retard, temps
  de décodage : une ligne `video:` toutes les deux secondes de lecture. Écarts entre
  présentations, images en avance ou en retard, temps de présentation : dans `frame account`.
- **Textures plus grossières.** Jusqu'à 6 niveaux en moins, plancher réglable
  (`textures:6 plancher:32`).
- **ScarlettPort étudié en entier.** Sa pile VIctory part du même fork que PROJECT ONE et peut
  être fusionnée ; la fusion est commencée sur la branche `project-one/scarlettport`
  (19 fichiers en conflit, 7 résolus). XCL (GPL-2.0 seule) n'est pas repris.

## 25/11/2026 — Series X : graphismes réduits et diagnostic par image

Détail, mesures et ordre d'essai : [docs/technical/07-graphismes-minimaux.md](docs/technical/07-graphismes-minimaux.md).
Rien de cette section n'a encore tourné avec PROJECT GAME.

- **Ligne `frame account` dans chaque version.** Toutes les cinq secondes : ce qui limite l'image
  (la carte, la préparation, le jeu), les parts en pourcentage, la mémoire vidéo et la RAM, les
  tailles de rendu demandées par le jeu et la taille présentée.
- **Textures réduites** (`lancer.cmd series-x textures:2`, `VICTORY_TEXTURE_SKIP_MIPS`). Les
  textures compressées du jeu sont copiées vers la carte sans leurs 1 à 4 premiers niveaux de
  détail, jamais sous 128 pixels. Désactivé par défaut.
- **Effets laissés de côté par nom** (`lancer.cmd series-x sans:vent+particules`,
  `VICTORY_SKIP_SHADERS`). Seules des passes de calcul sont laissées de côté, jamais un dessin :
  aucun modèle ne peut disparaître. Les groupes sont des fichiers dans `..\victory-profils`.
  `lancer.cmd series-x minimal` regroupe `textures:2` et les groupes à faible risque.
- **Noms des shaders.** `preparer` écrit la liste des 25 894 shaders nommés du jeu à côté du
  cache (`shader-names.txt`) ; `victory prepare --names-only` écrit la liste seule.
- **Définition.** Établi par les captures : 2560 × 1440 en interne et 3840 × 2160 présentés en
  mode Series X, sans définition dynamique ni paramètre. Un vrai 720p demande le profil Series S
  du jeu ou une mise à l'échelle par le runtime, non commencée.
- **Mode Series S** : la correction de la mémoire n'a pas suffi, le jeu reste sur son premier
  écran (lancement de 7 minutes). Piste mise de côté.

## 24/11/2026 (soir) — performances

Diagnostic complet et mesures : [docs/technical/06-performances.md](docs/technical/06-performances.md).

- **Mode Series S : une cause corrigée, le blocage persiste (vérifié le 25/11).** Le runtime disait au jeu qu'il utilisait toute
  la mémoire du processus (6 082 Mo sur 7 776 Mo trois secondes après le démarrage). Le jeu, qui
  ne pose cette question qu'en mode Series S, ne chargeait rien après son premier écran. La
  réponse compte maintenant la mémoire du jeu seul. `VICTORY_TITLE_USED=process` remet l'ancienne.
- **Dessins mesh indirects.** Les 2 000 à 2 260 dessins mesh indirects par image (la géométrie
  solide du monde) étaient sautés. Ils sont exécutés quand l'appel porte un seul enregistrement,
  ce qui est le cas du jeu. Les appels à plusieurs enregistrements restent sautés.
- **Un seul tampon d'enregistrements mesh**, de 32 Mio, pour toutes les listes
  (`VICTORY_MESH_RECORDS_MB`). Avant, chaque liste pouvait garder le sien, jusqu'à 8 Mio.
- **Ordre de compilation des pipelines.** Les pipelines demandés par l'image en cours passent
  avant ceux demandés plus tôt et plus utilisés.
- **Commande `prepare`.** `victory prepare --compute-pipelines` traduit tous les shaders du jeu
  dans le cache et compile ses pipelines de calcul avant le lancement. Mesuré : 25 466 shaders
  et 2 723 pipelines en 9 min 41 s. Avec `--mesh-pipelines`, elle compile aussi le pipeline de
  calcul de chaque programme mesh : 8 762 programmes, 0 refusé, en 2 min 25 s.
- **Cache de shaders rangé par version.** Le cache garde la version en cours et la précédente,
  et retire les autres. Avant, chaque recompilation ajoutait 1,3 Go pour toujours.
- **Lanceur.** Modes `profil` (temps de la carte par commande) et `preparer`, un journal par
  lancement dans le dossier `victory-logs`, à côté du dépôt.
- **Banc d'essai** du schéma de dessin mesh (`crates/victory-gpu/src/bench.rs`) : le schéma
  actuel coûte 2 à 5 ms par image, il n'est pas réécrit.

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

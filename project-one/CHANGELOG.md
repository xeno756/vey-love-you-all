# Changelog de PROJECT ONE

Branche `project-one/integration` du fork de VIctory par fxceless. Étiquettes :
- `project-one/baseline` : le fork tel que cloné (`7b0f736`) ;
- `project-one/upstream-a803c87` : le fork à jour sur lequel PROJECT ONE est rebasé.

## 25/11/2026 (19 h 30) — le fork VIctory-7900XTX essayé

Détail et mesures : [docs/technical/11-victory-7900xtx.md](docs/technical/11-victory-7900xtx.md).

- **Pas utilisable tel quel.** Il dessine les programmes mesh en vrais mesh shaders, que la
  RX 5600 XT n'a pas, et prend 40 à 46 Go de mémoire ; sa fusion touche 46 fichiers déjà
  réécrits ici.
- **Trois correctifs repris puis mesurés en jeu.** Aucun gain de vitesse. Deux pertes de
  carte sur quatre lancements avec eux, aucune sur deux sans : les deux correctifs actifs sont
  retirés (`df2da97`, `663d05c`) en attendant de les mesurer un par un.
- **Lecture des pixels voisins.** Celle du fork fait planter le compilateur de shaders du
  pilote AMD : coupée par défaut (`83c3e60`, `VICTORY_QUAD_READS`), la précompilation des
  pipelines graphiques repasse sans échec.
- **L'image varie d'un lancement à l'autre** avec le même code (mosaïques bleues) : ce n'est
  pas dû aux reprises (BLOCKERS 15).

## 25/11/2026 (11 h 20) — mesure par image, textures qui ne restent plus bloquées, `maximum` réparé

Suivi commit par commit : [PERF_JOURNAL.md](PERF_JOURNAL.md). Décisions prises sans
question : [DECISIONS.md](DECISIONS.md). Ce qui reste bloqué : [BLOCKERS.md](BLOCKERS.md).
Aucun de ces changements n'a encore tourné avec PROJECT GAME.

- **Ce que vos trois lancements de 10 h ont mesuré.** `rapide` : la carte passe de 777 à
  221 ms par image, 67 915 textures chargées en 2 minutes au lieu de 6 596 en 7 min 35, la
  carte reste dans son budget ; environ 2,2 images par seconde à la fin. `maximum` : resté
  sur l'écran des licences, à cause de la taille 960 × 540.
- **Textures (`dfca70d`).** Elles ont maintenant leur propre réserve sur la carte : plus de
  blocage quand le reste dépasse le budget. Les copies deviennent plus grossières à mesure
  que la réserve se remplit, et quelques-unes sont faites à chaque image même pleine.
- **Mesure (`7a95d07`).** Un fichier `.images.csv` par lancement, une ligne par image ; la
  barre de titre donne la médiane et le 99e centile des 300 dernières images, la mémoire de
  la carte et les textures en attente. Les capacités de la carte sont écrites au démarrage.
- **Pipelines refusés (`ee2d30b`).** Le journal dit quelle capacité manque : les
  barycentriques, que la RX 5600 XT n'a pas (lu sur la carte).
- **Profils (`fd37912`).** `maximum` demande 1280 × 720. `rapide` ne laisse plus aucun effet
  de côté : 1280 × 720 et textures allégées seulement.

## 25/11/2026 (10 h) — premier lancement réussi de la branche fusionnée, profils `rapide` et `maximum`

Mesures et profils : [docs/technical/10-premier-lancement-reussi-et-profils.md](docs/technical/10-premier-lancement-reussi-et-profils.md).

- **La correction de 9 h 30 est confirmée en jeu** : le monde s'affiche (lancement de 9 h 41,
  7 min 35 s dans le monde, 0,2 à 3,1 images par seconde).
- **Ce lancement s'est terminé par manque de mémoire**, et n'a chargé qu'une texture sur
  sept : la carte dépasse son budget de 2 Go dès la première image du monde en 2560 × 1440.
- **Profils ajoutés au lanceur de la branche fusionnée**, pas encore essayés en jeu :
  `rapide` (1280 × 720, textures 16 fois plus légères, 4 groupes d'effets en moins),
  `maximum` (960 × 540, textures 64 fois plus légères, 8 groupes sur 9), et `vitesse` pour
  les réglages de ScarlettPort du côté du processeur.
- **Pipelines de dessin du jeu compilés d'avance** avec la version corrigée : 24 801 sur
  25 471, en 58 s.

## 25/11/2026 (9 h 30) — la branche fusionnée perdait la carte au monde : une erreur de fusion

Cause et mesures : section 4 de [docs/technical/09-plantages-du-25-11.md](docs/technical/09-plantages-du-25-11.md).
Cette correction n'a pas encore tourné avec PROJECT GAME.

- **Ce qui a cassé l'entrée du monde, c'est ma fusion.** ScarlettPort donne à 94 % des shaders
  du jeu une table que le runtime doit leur brancher à l'emplacement 252. Notre code des
  programmes mesh utilisait ce même numéro et empêchait ce branchement : la table n'était
  branchée pour aucun shader. Le tampon mesh passe à l'emplacement 251, et la compilation
  refuse désormais que les deux numéros se rejoignent.
- **Le programme AMD laissé de côté le matin n'était pas la cause** de la perte de la carte
  (elle a continué sans lui). Il reste laissé de côté : la mémoire qu'il prend est réelle.
- **La préparation ne servait pas au jeu** sur cette branche : le jeu rangeait ses shaders
  dans un autre dossier que la préparation et retraduisait tout. Corrigé.
- La branche stable n'est pas touchée : elle n'a jamais eu cette table, ni ce défaut.

## 25/11/2026 (matin) — les trois défauts de vos essais

Causes et preuves : [docs/technical/09-plantages-du-25-11.md](docs/technical/09-plantages-du-25-11.md).
Aucune de ces corrections n'a encore tourné avec PROJECT GAME.

- **`1080p` et `720p` se fermaient au démarrage.** Le jeu vérifie que ses images de sortie
  utilisent la mémoire qu'il leur a réservée à sa taille d'origine, et s'arrête sinon. Cette
  vérification ne l'arrête plus ; toute autre vérification du jeu rencontrée avec une taille
  imposée est franchie et écrite dans le journal.
- **La branche fusionnée perdait la carte à l'entrée du monde.** Le pilote AMD ne finit pas de
  compiler un programme de l'éclairage indirect et prend 0,8 Go de plus toutes les 5 à 10
  secondes (reproduit hors jeu). Ce programme n'est plus compilé sur une carte AMD. La
  préparation de cette branche passe de 18 min 47 s à 21 s, cache chaud.
- **L'introduction saccadait sur la branche stable.** Ce n'était pas le calage sur l'écran (le
  vôtre est à 280 Hz et les images sortaient bien toutes les 33,3 ms) : le runtime ne disait
  pas au jeu quand ses images étaient affichées. La branche fusionnée le disait déjà ; la
  branche stable le dit maintenant aussi.
- **Deux vidéos en même temps ?** Non. Le compteur vidéo montre que la séquence d'avant le logo
  n'est pas un film, et qu'un seul film est lu à la fois.
- **La meilleure version** est la branche fusionnée avec ces corrections, dans
  `..\victory-scarlettport`.

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
- **ScarlettPort étudié en entier et fusionné.** Sa pile VIctory part du même fork que
  PROJECT ONE. Elle est fusionnée sur la branche `project-one/scarlettport` : 20 fichiers en
  conflit résolus, leur traducteur repris tel quel avec nos programmes mesh en calcul par-dessus,
  une seule commande de préparation, 752 tests réussis. Pas encore essayée en jeu, donc pas
  encore dans la branche stable. XCL (GPL-2.0 seule) n'est pas repris.

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

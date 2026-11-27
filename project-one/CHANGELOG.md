# Changelog de PROJECT ONE

Branche `project-one/integration` du fork de VIctory par fxceless. Étiquettes :
- `project-one/baseline` : le fork tel que cloné (`7b0f736`) ;
- `project-one/upstream-a803c87` : le fork à jour sur lequel PROJECT ONE est rebasé.

## 26/11/2026 (21 h – 22 h 15) — un verrou par groupe de cases

- `f5fc552` (branche fusionnée) : les cases du tas de vues se font sous le verrou de leur
  groupe de 16 cases (4 096 verrous) au lieu d'un seul verrou pour tout le tas. La
  synchronisation des descripteurs l'attendait 87,5 ms par image sur le fil d'envoi ; elle
  l'attend 12 ms. Partie de 10 min (21 h 58) : **4,91 images/s** au lieu de 4,41, médiane
  200 ms au lieu de 224, 99 % sous 309 ms, carte gardée. `VICTORY_SLOT_LOCKS=0` : l'ancien
  verrou.
- `c9cd850` : les pages changées des tas réparties sur 4 fils par défaut
  (`VICTORY_SYNC_THREADS`, 1 : le fil d'envoi seul) : la synchronisation prend 45 ms par image
  au lieu de 93. Partie de 7 min (22 h 11) : **5,29 images/s**, médiane 183 ms, carte gardée ;
  mais la partie de 20 min de 22 h 47 a perdu la carte vers 14 min (5,4 images/s).
- `b9764f5` : retour à un fil par défaut ; 20 min à 23 h 20 : **4,95 images/s**, médiane
  200 ms, carte gardée. Option `VICTORY_SYNC_AHEAD=1` (la synchronisation pendant la
  préparation des listes, non mesurée).
- `9829e33` puis `bfa2b73` : essai de prendre le plantage du jeu `title+0x4dbd74d` (vu à 23 h 32
  avec la version par défaut) comme un cache vide : le jeu s'arrête juste après ailleurs (le
  contexte principal n'a pas d'anneau d'envoi) ; éteint par défaut. Le plantage reste possible
  (doc 14).
- Écarté : constantes des tables lues dans la copie sur la carte (carte perdue), copies et
  instantanés sur plusieurs fils du fork amont (rien), 8 fils (carte perdue à 3 min).
- Le plantage du jeu `title+0x4dbd74d` est expliqué (doc 14) ; la couche de débogage de
  Direct3D ne trouve aucune faute côté processeur dans les pertes de carte.

## 26/11/2026 (17 h 25 – 21 h 35) — nuit de travail seul, parties lancées par moi

Une quarantaine de parties (`banc.ps1`), captures de la fenêtre et une capture de diagnostic
des tampons de rendu de la cinématique d'ouverture.

- **Stable par défaut** : ~4,5 à 5 images/s, 12 min sans perte de carte (`d951587`).
- `3052ba9`, `39f8fca` : clés des cases lisibles sans verrou, table des vues en lecture
  partagée ; le marquage des cases de table (`VICTORY_TABLE_SLOTS=1`) donne 6,3 à 7 images/s
  mais perd la carte (4 fois en 16 min) : en option.
- `d951587` : lumière ambiante 0,15 (cellule éclairée naturellement) ; les textures de
  personnages refaites plus fines faisaient un bruit blanc sur la peau : en option
  (`VICTORY_DETAIL_TEXTURES=1`).
- `01cfafa` : les textures choisies par index sont créées en parallèle (débit doublé).
- **Visages noirs en cinématique : pas résolu.** Les textures de la peau sont prises par index
  depuis les constantes du matériau ; environ 19 000 textures attendent que le jeu charge leur
  mémoire.
- Écarté après mesure : port #1 (le tas de 174 Mo est en 3 vues hors ordre), `vitesse:1`,
  lumière indirecte gardée (magenta), profil console Series S (mémoire insuffisante).

## 26/11/2026 (17 h 20) — le port #2 retiré de `performance`

Partie de l'utilisateur de 16 h 41 (14 min), avec le port #2 :

- **5,1 images par seconde** en régime établi (médiane 181-185 ms), contre 4,4 (224-234 ms) à
  14 h 56. Synchronisation des tas : 43 à 129 ms par image au lieu de 98 à 110.
- Mais son filet trouve **150 à 450 descripteurs par image** sur ses 16 pages qu'aucune marque
  n'a signalés : le jeu en écrit par d'autres chemins que les 6 patchés, et son `memcpy`
  système n'en marque aucun. Ces cases gardent une vue périmée plusieurs secondes.
- **La carte est perdue à 14 min** (défaut de page de la carte pendant la passe des vues de
  tampons) : la première perte avec le relais 1 depuis 10 h 38. Lien avec le port non prouvé.
- Mémoire : 369 Mo libres sur 15,8 Go en fin de partie ; 6,6 Go de mémoire partagée avec la
  carte ; Discord et Edge tiennent environ 3 Go.

Retiré de `performance` (`255c2ba`), gardé sous le mot `marques`. Le code reste, éteint par
défaut.

## 26/11/2026 (16 h 40) — le jeu signale lui-même ses descripteurs (port #2)

- Partie de 14 h 56 : 4,47 images par seconde, médiane 220 ms. La synchronisation des tas de
  descripteurs coûte **110 ms par image** sur le fil qui soumet : 254 pages relues,
  519 000 cases comparées, 61 000 refaites.
- Repris du fork d'avatarmanz (VIctory-7900XTX, GPL-3.0-or-later, `4cb90be`), son « port #2 » :
  les 6 écritures du jeu qui remplissent ses tables de descripteurs (vérifiées octet pour octet
  dans notre exécutable) et son `memcpy`/`memmove` marquent les cases écrites ; la
  synchronisation ne relit plus que ces cases. Un filet relit 16 pages entières par image et
  compte ce qu'aucune marque n'a signalé (`ea371e3`).
- Dans `performance` (étiquette `-marques`). `VICTORY_PORT_DESCRIPTORS=0` avant le lancement
  le retire. **Jamais lancé chez nous.**

## 26/11/2026 (15 h) — limiteurs, lancer de rayons, plantage, personnages

Détail : [docs/technical/13-limiteurs-rayons-personnages-26-11.md](docs/technical/13-limiteurs-rayons-personnages-26-11.md).

- Lancement de 14 h 01 : 4,07 images par seconde avec les murs ; la carte ne travaille que 10 %
  de l'image ; plantage du jeu après 9 minutes (une première).
- Pas de plafond qui retienne le monde : le jeu demande 30 images/s, ce qui ne joue que sous
  33 ms par image. Réglages `cadence:` et `vsync:` ajoutés, éteints.
- Lancer de rayons : déjà éteint chez nous ; aucun gain possible de ce côté aujourd'hui.
- Plantage : les écritures du noyau dans la mémoire du jeu se font gardes tenues ouvertes.
- Personnages : leurs textures gardent un niveau de détail de moins retiré au lieu de trois
  (`personnages`, dans `performance`). Non vérifié.

## 26/11/2026 (14 h 30) — les murs sont revenus ; la vitesse a baissé

- Capture de l'utilisateur à 13 h 47 : **le dortoir a ses murs**.
- Mais 3,8 images par seconde à 90-130 s de monde (6,7 avant) : le jeu copie 58 000
  enregistrements de descripteurs par image, que nous refaisons maintenant, et la passe des
  vues de tampons en paie le prix. Compteurs ajoutés pour savoir lesquels (`762c7b8`).

## 26/11/2026 (14 h) — les murs ; 6,4 images par seconde

Détail : [docs/technical/12-audit-moteur-26-11.md](docs/technical/12-audit-moteur-26-11.md), section 9.

- Mesuré (lancement de l'utilisateur à 13 h 14) : **6,43 images par seconde**, médiane 153 ms,
  avec `attentes-textures`, que `performance` prend désormais. Le visage de l'héroïne est texturé.
- **Les murs du dortoir manquaient** (défaut connu depuis le 25/11) : les deux correctifs de
  notre amont qui lui ont rendu ses murs sont repris (`a5b2dd3`) : les descripteurs que le jeu
  copie en octets dans ses tas sont recopiés, et une écriture par une vue d'une page est vue par
  ses autres vues. **Non vérifié en jeu.**

## 26/11/2026 (13 h) — le monde s'affiche ; visages noirs, soleil à travers les murs, attentes de la carte

Détail : [docs/technical/12-audit-moteur-26-11.md](docs/technical/12-audit-moteur-26-11.md).

- **Le monde est visible** depuis que le brouillard est rendu : lancement de l'utilisateur
  à 11 h 12, 5,6 images par seconde, médiane 175 ms, 300 s sans perte de carte.
- **Visages noirs** (déjà vus le 25/11) : deux défauts du chargement des données, corrigés par
  notre amont fxceless et repris ici (`84fe251`). Les lectures DirectStorage vers des pages
  physiques ne partent plus avant que le jeu n'ait rempli leur liste de pages, et les textures
  sous une lecture terminée sont réexaminées. **Non vérifié en jeu.**
- **Le soleil traversait les murs** (aucune carte d'ombres) : `performance` dessine de nouveau
  la cascade d'ombres la plus proche (`b3dd536`). Profils A `ultra-bas`, B `performance`,
  C `equilibre`. Non mesuré.
- **Essai** `attentes-textures` (`3018756`, éteint par défaut) : 11,8 des 13,8 attentes de la
  carte par image venaient du tampon d'envoi d'une texture ; il n'attend plus que la
  soumission qui le lit. À mesurer en deux lancements.
- Piste `ifeyz/champion4` : introuvable en public ; ses variables viennent des forks
  7900XTX et Belobug ; aucune reprise telle quelle.
- Notre amont fxceless a 98 commits d'avance (images justes dans la première pièce, visages,
  plusieurs gains mesurés) : fusion à faire en phase à part.

## 26/11/2026 (11 h 15) — le monde noir : le brouillard rendu de nouveau ; préréglage SafeMode

Détail : [FINDINGS.md](FINDINGS.md), « Le monde noir ».

- **Pourquoi le monde était noir de 8 h 25 à 10 h 38 :** `performance` laissait de côté la
  collecte du brouillard volumétrique (`sans:volumetrie-collecte`), ajoutée sans vérifier
  l'image. La passe qui applique le brouillard lit alors des textures que rien n'écrit. Les
  journaux le montrent : ce groupe est laissé de côté dans tous les lancements depuis 8 h 25,
  et dans aucun avant (8 h 13 : l'utilisateur voyait les modèles). Cause probable, pas
  démontrée à l'image.
- **`performance` rend de nouveau le brouillard** (`2273cfe`, branche fusionnée). **Non
  vérifié en jeu.** Les 6 images par seconde du matin étaient mesurées sans brouillard,
  donc sur un monde noir : elles ne comptent pas. Avec le brouillard, la vitesse n'est pas
  mesurée.
- **Préréglage du jeu** (`2cf979c`, branche fusionnée) : `prereglage:bas`, `extreme`,
  `diagnostic` ou `safemode`. Le jeu prend son préréglage SafeMode au lieu du sien, puis
  certains réglages sont abaissés avant qu'il ne l'applique (qualités au plus bas, flou de
  mouvement coupé, niveau de détail et herbe réduits). Éteint par défaut, **jamais lancé**.
- Je ne lance plus le jeu : l'utilisateur fait tous les lancements (DECISIONS 35).

## 26/11/2026 (10 h 45) — `performance` tient : 6 images par seconde, deux fois 300 s

Détail : [FINDINGS.md](FINDINGS.md), « Correction (10 h 45) ».

- **La vraie cause des pertes de carte :** le relais au niveau 2 avec les attentes précises,
  entrés dans les profils à 8 h 13. Avec eux, 10 lancements sur 15 ont perdu la carte ; sans
  eux, 1 sur 14. L'entrée de 10 h 10 accusait `vitesse:3` : c'était incomplet.
- **`performance` les retire** : relais au niveau 1, sans attentes précises, avec tout le
  reste. Deux lancements de 300 s tenus : 6,1 et 6,0 images par seconde, 95 % des images sous
  190 ms environ, la meilleure régularité de la journée.
- Les réglages plus rapides mais instables restent des options, signalées dangereuses
  dans le lanceur (`relais:2 precis`, `vitesse:2`, `vitesse:3`).

## 26/11/2026 (10 h 10) — `performance` stable : 6,3 images par seconde (corrigé à 10 h 45)

Détail : [FINDINGS.md](FINDINGS.md), « `vitesse:3` perd la carte » ; mes lancements dans
[PERF_JOURNAL.md](PERF_JOURNAL.md).

- **Les pertes de carte venaient de `vitesse:3`.** Avec lui, 6 lancements sur 7 ont perdu la
  carte ; sans lui, aucun sur 4. Il allait plus vite (8,8 images par seconde) parce qu'il ne
  recopiait plus vers le jeu ce que la carte calcule, et c'est justement ce qui finit par
  donner au jeu des adresses fausses. Il est retiré de `performance`.
- **Ce qui a été regagné sans lui :** 256 listes en vol au lieu de 128 ; des fenêtres de
  mémoire qui n'attendent plus pour rien ; et nos propres passes de contrôle sur la carte,
  réécrites pour lire chaque morceau à 64 fils au lieu d'un seul. De 5,3 à 6,3 images par
  seconde.
- **Retiré :** un essai pour que les dessins mesh prennent leurs arguments directement
  (deux pertes de carte, aucun gain).
- La version à essayer reste une seule : `series-x performance`.

## 26/11/2026 (9 h) — `performance` : une seule version, 8,8 images par seconde, la carte tient

Détail : [FINDINGS.md](FINDINGS.md), cycles 3 et 4 ; mes lancements dans
[PERF_JOURNAL.md](PERF_JOURNAL.md).

- **Une seule version à essayer : `series-x performance`.** Elle réunit tout ce qui a été
  mesuré plus rapide. `performance-sur` garde le réglage d'avant (5 images par seconde).
- **Mesuré (mon lancement de 8 h 54, 300 s de monde) :** 8,77 images par seconde, médiane
  110 ms, 95 % sous 162 ms, contre 4,5 le matin et 3 avec `rapide`. Les objectifs de 10 et
  15 images par seconde ne sont pas encore atteints.
- **Perte de carte corrigée, à confirmer.** Le lancement de 8 h 25 avait perdu la carte à
  90 s : des objets libérés étaient encore lus, parce que la marge de deux images ne dure
  plus que 230 ms quand le jeu va vite. Ils attendent maintenant une seconde de plus. Un
  lancement de 300 s a tenu ; il en faut d'autres.
- **Chargement.** La création des textures est bornée à 80 ms par image : la première
  minute tourne à 4–6 images par seconde au lieu de 2–3, et les textures arrivent plus tard.
- **Je lance de nouveau le jeu moi-même** pour mesurer (DECISIONS 29).

## 26/11/2026 (8 h 25) — attentes précises mesurées : 5 images par seconde ; cycle 3 prêt

Détail : [FINDINGS.md](FINDINGS.md), « Cycle 2, mesuré » et « cycle 3 ».

- **Mesuré.** `performance precis relais:2` : 4,97 images par seconde (médiane 187 ms),
  sans alerte ni perte de carte. Le processeur n'attend plus la carte que 4 fois par image
  au lieu de 13, mais ces attentes durent encore 119 ms.
- **Pourquoi.** Elles attendent la relecture de ce que les shaders écrivent sur la carte :
  36 Mo par image.
- **Suite.** `vitesse:2` limite cette relecture à ce que le jeu relit ; `vitesse:3` joint
  en plus les passes de contrôle du runtime sur la carte. À lancer.
- **Banc.** Une ligne « recopie » de plus par lancement.

## 26/11/2026 (8 h 15) — `performance` mesuré : 4,5 images par seconde ; cycle 2 prêt à lancer

Détail : [FINDINGS.md](FINDINGS.md), « Cycle 1, mesuré » et « cycle 2 ».

- **Mesuré.** `performance`, deux lancements en banc propre : 4,68 et 4,45 images par
  seconde (201 et 217 ms par image) contre environ 3 avec `rapide`. Aucune perte de carte.
  Les objectifs de 10 et 15 images par seconde ne sont pas atteints.
- **Pourquoi ça reste lourd.** Le processeur attend la carte 13 à 14 fois par image
  (135 à 154 ms sur 200) : les deux travaillent encore l'un après l'autre.
- **Trois réglages de plus dans le lanceur**, à donner après le profil, aucun mesuré :
  `precis` et `relais:2` (contre ces attentes), `fenetres` (moins de passes par dessin).
  Rien n'est recompilé.
- **Personnages incomplets.** Le haut du corps du personnage proche de la caméra manquait.
  Cause probable : le groupe `transparence`. `performance` ne le laisse plus de côté.
- **Banc.** Une ligne de plus par lancement : attentes de la carte par image, part du temps
  où la carte travaille, alertes des attentes précises.

## 26/11/2026 (matin) — mission 10 à 15 images par seconde : audit, profil `performance`

Détail : [FINDINGS.md](FINDINGS.md), cycle 1. **Rien de cette entrée n'est mesuré en jeu** :
je ne lance plus le jeu, les mesures viendront des séries du banc que tu lances.

- **Audit.** Départ à 2,7 images par seconde (330 ms par image). La carte passe 136 de ses
  189 ms dans 4 549 dessins de programmes mesh, dont au moins 1 300 pour les cartes d'ombres.
  Côté processeur : 53 000 défauts de page par image, la machine manque de RAM.
- **Cartes d'ombres laissées de côté.** Option `sans-ombres` : le monde reste éclairé, sans
  ombres portées. `sans-dessins:<groupe>` fait de même pour un groupe de ton choix
  (`..\victory-profils\dessins-<groupe>.txt`).
- **Profil `performance`.** `maximum`, plus les ombres écartées, le relais et les réglages
  bas du jeu. À mesurer contre `rapide`.
- **Mesure.** Le fichier par image compte les dessins écartés, les défauts de page et le
  temps processeur du processus. Le banc affiche les images par seconde et dit, pour chaque
  lancement, si les objectifs 10 et 15 images par seconde sont atteints.
- **Attendu, sans preuve.** Autour de 220 à 250 ms par image avec `performance`, soit 4 à
  4,5 images par seconde : ce cycle seul ne donne pas 10 images par seconde.

## 26/11/2026 (1 h 30) — banc propre, relais remesuré, veille installée

Détail : [FINDINGS.md](FINDINGS.md), itération 4. État de chaque lot : [LOT_LEDGER.md](LOT_LEDGER.md).

- **Banc.** `banc.ps1 -Propre` refuse de mesurer quand un autre programme tient plus de 100 Mo
  de la carte ; `-Series N` donne une ligne par lancement ; chaque lancement laisse un fichier
  `.env.json` (pilote, horloge, température et activité de la carte). La carte n'est ni bridée
  ni saturée : elle n'est active qu'à moitié, elle attend le processeur.
- **Relais.** Remesuré maintenant que la pâte bleue est corrigée : image propre, 54 ms de
  moins par image (environ 14 %). Il reste une option (`relais`), pas le défaut.
- **Un défaut nouveau à l'image.** Les murs de la cellule manquent dans 3 captures sur 7,
  avec ou sans relais. Pas encore expliqué.
- **Pool de tampons d'envoi.** Un défaut trouvé à la lecture est corrigé (une texture pas
  encore chargée pouvait montrer les texels d'une autre au lieu de noir).
- **Veille des dépôts.** `veille.ps1`, une passe par heure : un résumé de ce qui change dans
  les dépôts suivis et de ce qui touche nos blocages. Elle ne télécharge ni ne fusionne rien
  et se met en pause quand le jeu tourne. `veille.ps1 -Desinstaller` la retire.

## 25/11/2026 (23 h 45) — pertes de carte contournées, pool de tampons d'envoi

Détail : [FINDINGS.md](FINDINGS.md), chantier P0. Inventaire des fichiers reçus :
[SDK_INVENTORY.md](SDK_INVENTORY.md).

- **Pertes de carte.** Deux lancements de suite ont perdu la carte ; les rapports nomment des
  objets détruits que la carte lisait encore. Toute destruction attend maintenant 2 images de
  plus : aucune perte sur les 10 lancements suivants, dans les mêmes conditions.
- **Tampons d'envoi des textures.** Découpés dans quelques grands tampons gardés au lieu d'un
  par texture : 17 fois moins cher à obtenir, vérifié par un mode « poison » qui rendrait
  toute lecture trop tardive visible en magenta.
- **Arrivée des textures.** `rapide` et `maximum` n'utilisent plus le rattrapage : deux fois
  moins d'images de plus de 0,8 s, au prix d'un monde habillé un peu plus tard pendant sa
  première minute et demie.
- **Mémoire partagée.** Elle est faite de copies de mémoire du jeu par fenêtres de 4 Mio
  (3,5 à 4,9 Go), pas de tampons d'envoi (0,1 Go). Le manque reste ouvert.
- **Le « SDK » reçu** décrit les structures des fichiers de données du jeu ; ce n'est pas un
  SDK de shaders et rien n'en est repris.

## 25/11/2026 (21 h 40) — pipelines mesurés, cause des images longues trouvée

Détail : [FINDINGS.md](FINDINGS.md).

- **Pipelines.** Le fichier par image dit maintenant combien de temps les images attendent
  des pipelines, combien sont faits et traduits pendant le jeu. Dans la scène mesurée : rien.
  Ils ne freinent pas l'image tant que `preparer` et `preparer-graphiques` ont tourné.
- **Images de 1,5 s dans les deux premières minutes du monde.** C'est le rattrapage des
  textures : chaque passe crée une copie et un tampon d'envoi par texture, 72 % de son temps.
  Sans rattrapage, le 99e centile baisse d'un tiers mais deux fois moins de textures
  arrivent ; le défaut ne change pas tant que l'image n'a pas été vérifiée.
- **Mémoire partagée.** Une capture s'est arrêtée faute de mémoire partagée avec la carte
  (7,7 Go, la limite de ce PC). À surveiller.

## 25/11/2026 (21 h) — la pâte bleue trouvée et corrigée

Détail et preuves : [FINDINGS.md](FINDINGS.md).

- **Cause.** L'éclairage du jeu lisait des textures de lumière rebondie que rien n'écrivait
  (l'éclairage indirect est laissé de côté, le lancer de rayons n'est pas exécuté). Elles
  gardaient ce que la mémoire contenait, souvent une image plus ancienne : d'où les blocs
  bleus, différents à chaque lancement. Prouvé en les remplissant de magenta : la pâte devient
  magenta.
- **Correctif.** `rapide` et `maximum` les remplissent d'une lumière faible et constante :
  plus de pâte sur deux lancements, aucun coût mesurable. `VICTORY_AMBIENT_FILL=off` le coupe.

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

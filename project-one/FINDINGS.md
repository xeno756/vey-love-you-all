# Constats

Ce qui a été établi par mesure, avec la preuve. Les captures sont hors du dépôt, dans
`..\victory-captures\<dossier>` ; les journaux dans `..\victory-logs`.

## Chantier A — la pâte bleue (25/11, 20 h à 21 h)

**Symptôme.** Sur certains lancements de `rapide`, des blocs bleu-blanc couvrent les murs, le
lit et les visages, sur le pourtour de l'image ; le centre (la cour vue par la fenêtre) reste
propre. Vu de près, la pâte contient une image : des bâtiments et la cour, teintés de bleu.

**Mesure utilisée.** Part des pixels nettement bleus (B > R + 40 et B > 110) des images du
monde enregistrées par le runtime :

| Lancement (version diagnostic, `rapide`) | Captures | Pâte (moyenne, maximum) |
|---|---|---|
| 17 h 12 | `essai-c1` | 2,2 %, 3,4 % |
| 18 h 48 | `essai-7900` | 4,4 %, 4,6 % |
| 19 h 25 | `essai-7900-retire` | **16,9 %, 23,7 %** |
| 20 h 21 | `a1-cibles` | 0,0 %, 0,0 % |
| 20 h 26 | `a1-cibles-2` | 2,8 %, 6,3 % |

La pâte va et vient d'un lancement à l'autre avec le même code et les mêmes réglages.

### A0 — le test discriminant

- Le runtime crée ses copies de textures en ressources « committed », que D3D12 remet à zéro,
  et son code le note : une texture pas encore envoyée s'affiche **noire**. Une donnée jamais
  arrivée ne peut donc pas donner du bleu par ce chemin.
- Le test appliqué aux textures suspectes (section A1) : `VICTORY_AMBIENT_FILL=1,0,1` les
  remplit de magenta avant que l'éclairage les lise. Lancement de 20 h 42 (`a0-magenta`,
  image 1501) : **les murs, le lit, l'encadrement et la tête deviennent magenta**, exactement
  les zones de la pâte ; la cour au centre ne change pas.
- **Branche prouvée : données jamais écrites** (A4 plus que A2). Ces textures sont placées
  dans la mémoire du jeu, et les passes qui les écrivent ne tournent pas : l'éclairage
  indirect, que `rapide` laisse de côté, et le lancer de rayons, que le runtime n'exécute pas.
  L'éclairage lit donc ce qui traîne à ces adresses (souvent une image rendue plus tôt, d'où
  la cour visible dans la pâte), différent à chaque lancement.

### A1 — la passe qui produit ces pixels

- **G-buffer propre** dans une image avec pâte (20 h 26, image 2000) comme dans une image
  sans (20 h 21, image 1700) : albédo, cheveux compris, et paramètres de matériau justes. Ce
  n'est ni un décodage de texture ni un format : le décodage BCPack n'a signalé aucun échec
  (601 Mo décodés au lancement de 18 h 41).
- **Pas de reflets d'écran** : aucun de leurs shaders ne tourne dans ces deux images.
- **La passe** : l'éclairage différé par tuiles, un composite plein écran par matériau (vertex
  `1d7b30217b395cd8`, pixel `a2f4056b21115d43` et une quarantaine de variantes par
  matériau). Cibles : la scène en 1280 × 720 R16G16B16A16_FLOAT et une cible R32 ; mélange
  coupé ; profondeur testée, pas écrite. Ses entrées : le G-buffer, les ombres, et **6
  textures R9G9B9E5 de 1280 × 720 placées dans la mémoire du jeu** (lumière rebondie).
  Aucune passe ne les écrit dans les images enregistrées (lectures seules, `uav=false`).
- **L'hypothèse d'un masque de composite cassé, ou d'un pipeline de secours, est éliminée** :
  0 pipeline refusé, aucun pipeline de remplacement ; le composite fonctionne, c'est une de
  ses entrées qui n'est jamais écrite.

### Correctif

`project-one\lancer.cmd` : quand l'éclairage indirect est laissé de côté (profils `rapide` et
`maximum`), `VICTORY_AMBIENT_FILL=0.02` remplit ces textures d'une lumière rebondie faible et
constante (le réglage existait déjà dans le runtime, sans être utilisé).
`VICTORY_AMBIENT_FILL=off` le coupe.

| Lancement | Réglage | Pâte (moyenne, maximum) |
|---|---|---|
| 20 h 34 (`a4-remplissage-1`) | `VICTORY_AMBIENT_FILL=0.02` | **0,0 %, 0,0 %** |
| 20 h 37 (`a4-remplissage-2`) | `VICTORY_AMBIENT_FILL=0.02` | **0,0 %, 0,0 %** |

Deux lancements sans pâte, contre quatre sur cinq avec pâte sans le remplissage (maximum
au-dessus de 3 %) : c'est un indice fort, pas une statistique. Le mécanisme, lui, est montré par l'essai en magenta. Limite : les intérieurs
reçoivent une lumière rebondie constante, pas la vraie, qui demande le lancer de rayons.

**Vitesse** (version normale, 300 s de monde, mêmes plans) :

| Lancement | Médiane | 95 % sous | 99 % sous | Processeur + carte + recopie |
|---|---|---|---|---|
| 19 h 19, sans remplissage | 325 ms | 580 | 1 843 | 96 + 198 + 36 |
| 20 h 46, avec (lanceur par défaut) | 327 ms | 400 | 1 062 | 93 + 197 + 36 |

Le remplissage ne coûte rien de mesurable (6 textures de 3,7 Mo écrites une fois par seconde
par le processeur). L'écart du 99e centile ne vient pas de lui : dans les deux lancements,
les images longues forment une grappe entre 60 et 100 s de monde (19 puis 10 images de plus
de 0,8 s), côté processeur (276 à 335 ms en moyenne), sans pipeline en cours de création.
C'est le prochain poste du 99e centile (BLOCKERS 17).

## Chantier B1 — le débit des pipelines (25/11, 21 h 18)

Six colonnes ajoutées au fichier `.images.csv` : temps que les fils d'enregistrement ont
attendu des pipelines en cours (`pipe_wait_ms`), pipelines faits et leur temps de création
(`pipes_made`, `pipe_make_ms`), shaders traduits pendant le jeu (`shaders_translated`) et
trouvés dans le cache disque (`shader_cache_hits`), âge du plus vieux pipeline qu'un dessin
attend (`pipe_oldest_ms`).

Lancement de 21 h 18 (version normale, `rapide`, après `preparer` et `preparer-graphiques`) :

| | 60 premières secondes de monde | Ensuite (240 s) |
|---|---|---|
| Attente des pipelines par les images | 0 ms | 0 ms |
| Pipelines faits pendant le jeu | 6 (81 ms) | 5 (196 ms) |
| Shaders traduits pendant le jeu | 0 | 0 |
| File de pipelines, au plus | 0 | 1 |
| Plus vieux pipeline attendu | 0 ms | 0 ms |

**Les pipelines ne freinent pas cette scène** : tout est préparé avant le lancement. Les
« dessins laissés de côté » du fichier (environ 430 par image) sont ceux laissés exprès :
l'éclairage indirect de `rapide` et les passes qui demandent le lancer de rayons.

## La grappe du 99e centile (BLOCKERS 17)

**Cause prouvée** : le rattrapage des textures repris de victory-amd. Tant que plus de 2 000
emplacements attendent et qu'il reste du surplus (60 s au total, `VICTORY_TEXTURE_GATE_S`),
chaque passe de textures peut prendre jusqu'à 1,5 s et 512 Mo par image, sur le fil qui
enregistre l'image. Minutage ajouté au journal (ligne « texture pass, last 5 s ») : pendant
le rattrapage, 4 passes prennent 6 048 ms pour 4 746 copies de textures créées en 2 207 ms
et 5 071 tampons d'envoi créés en 2 116 ms. **72 % de la passe est de la création de
ressources** : une copie sur la carte et un tampon d'envoi propre à chaque texture.

| Lancement (version normale, 300 s) | Médiane | 95 % sous | 99 % sous | Images > 0,8 s | Textures envoyées à 60 / 120 / 240 s |
|---|---|---|---|---|---|
| 20 h 46, rattrapage par défaut | 327 ms | 400 | 1 062 | 48 | 53 953 / 70 963 / 72 254 |
| 21 h 18, rattrapage par défaut | 337 ms | 497 | 1 204 | 40 | 46 676 / 64 328 / 65 323 |
| 21 h 08, sans rattrapage (`VICTORY_TEXTURE_GATE_S=0`) | 332 ms | 448 | **754** | 11 | 23 713 / 37 170 / 38 798 |
| 21 h 27, passes de rattrapage de 500 ms (`VICTORY_TEXTURE_CATCHUP_MS=500`) | 330 ms | 397 | 813 | 37 | 29 217 / 39 372 / 40 709 |

Sans rattrapage, le 99e centile baisse d'un tiers, mais deux fois moins de textures sont
envoyées ; à 500 ms, à peine plus qu'au plus bas. La file ne se vide dans aucun cas :
environ 21 000 emplacements attendent que le jeu remplisse leur mémoire.

**Le défaut reste inchangé** (rattrapage de 1,5 s) : la capture qui devait montrer si le
monde reste habillé sans rattrapage s'est arrêtée sur un manque de mémoire (ci-dessous), et
aucun gain n'est annoncé sans cette preuve. Le vrai correctif est de rendre la passe moins
chère : réutiliser les tampons d'envoi au lieu d'en créer un par texture, ou créer les
copies hors du fil qui enregistre l'image. Ce n'est pas fait : un tampon d'envoi peut être
gardé par une liste indépendamment de son état, et le réutiliser trop tôt corromprait une
texture.

## Manque de mémoire partagée (25/11, 21 h 15)

Capture de la version diagnostic sans rattrapage : fermée après moins d'une minute de monde,
`0x8007000E` (« no shared buffer », « no upload buffer »). La mémoire partagée avec la carte
était à 7,7 Go, sa limite sur ce PC ; le budget de la carte était tombé à 4,3 Go (5,3 Go
d'habitude : d'autres programmes s'en servaient). Les lancements normaux y montent déjà à
6,8–7,3 Go. BLOCKERS 18.

## Chantier P0 (25/11, 22 h à 23 h 40)

Conditions de cette série : OBS, Discord et d'autres programmes ouverts tiennent 1,4 Go de la
carte ; son budget tombe à 3,9–4,4 Go et elle met 310 à 330 ms par image au lieu de 200. Les
chiffres de cette série ne se comparent qu'entre eux.

### P0.7 — la file de textures qui « ne se vide pas »

Composition de la file au fil d'un lancement par défaut (21 h 18, lignes du journal) :

| Temps de monde | Dans la file | Dues (du travail à faire) | En attente de leur mémoire |
|---|---|---|---|
| 15 s | 23 240 | 19 816 | 4 078 |
| 45 s | 23 385 | 5 469 | 19 473 |
| 75 s | 26 153 | 2 029 | 23 687 |
| 100 s | 21 633 | 165 | 20 950 |
| 3 à 4 min | 20 400–20 500 | 40–140 | 19 800–19 900 |

Les trois hypothèses sont écartées : ni fuite de compteur, ni budget, ni redemandes en boucle.
**La file est faite d'emplacements dont le jeu n'a pas encore chargé la mémoire** ; le travail
réel (« dues ») tombe sous 200 en 100 à 140 s, avec ou sans rattrapage. La colonne
`tex_waiting` du CSV comptait les deux ; `tex_due` et `tex_away` les séparent désormais.

Même constat pour « textures envoyées » : ce compteur compte aussi les renvois d'une texture
qui arrive par morceaux. La bonne mesure du monde habillé est le nombre de **textures
vivantes** (`host_tex_alive`) : environ 21 000 (3,1 Go) à 240 s dans tous les réglages.

### P0.1 — de quoi est faite la mémoire partagée

Ligne « memory » de la version diagnostic 5 s avant l'arrêt de 21 h 15 (7,7 Go partagés) :

| Poste | Vivants | Mémoire |
|---|---|---|
| Copies de mémoire du jeu, en tout | 4 857 | 5 803 Mo |
| dont fenêtres de 4 Mio hors tampons | 1 143 | **4 932 Mo** (2 475 Mo seulement de leurs pages jamais copiées) |
| Tampons d'envoi de textures | 301 | 81 Mo |
| Mémoire de travail des listes | 122 morceaux | 79 Mo |

**Les tampons d'envoi ne sont pas la cause du manque de mémoire** (81 Mo sur 7 700) : ce sont
les fenêtres de mémoire du jeu, des copies de 4 Mio dont la moitié des pages ne sert pas.
Douze colonnes ajoutées au CSV le suivent dans toute version : budget partagé, textures dues
et en attente, textures vivantes, tampons d'envoi, copies de mémoire du jeu, fenêtres, RAM
libre et engagement restant. Lancements normaux de la série : fenêtres 3,5 à 4,3 Go sur 6,4 à
7,0 Go partagés ; **RAM libre au plus bas : 3 à 58 Mo**.

### Pertes de carte : des objets détruits trop tôt (BLOCKERS 9)

Deux lancements de suite ont perdu la carte en moins d'une minute de monde (22 h 21, 22 h 23).
Les deux rapports nomment un objet **déjà libéré** que la carte lisait : un tampon de travail
de liste, puis une cible de rendu 1920 × 1088 (une image de vidéo). Même signature qu'à
19 h 07.

Cause : `retire` détruisait un objet « dès qu'aucune soumission ne tourne ». Or une liste que
le jeu a soumise peut être mise en attente et exécutée plus tard, et rien ne la comptait ; les
tampons de travail des listes, eux, étaient détruits sans attendre du tout.

Correctif : toute destruction attend en plus 2 images (`VICTORY_RETIRE_FRAMES`, 0 pour
revenir), et les tampons de travail passent par la même règle. **Avant : 2 pertes sur 2.
Après : 0 sur 10 lancements** dans les mêmes conditions (9 de 300 s, 1 de 240 s). Dix
lancements ne prouvent pas que la cause est la bonne : ils montrent que la panne ne revient
plus dans ces conditions.

### P0.2 / P0.3 — pool de tampons d'envoi

La passe de textures créait un tampon d'envoi par texture, utilisé une fois. Ils sont
maintenant découpés dans quelques tampons de 8 Mio gardés (`crates/victory-gpu/src/staging_pool.rs`).
Un morceau n'est redonné que lorsque plus aucun bail n'y vit **et** que toutes les
soumissions commencées avant la fin du dernier bail sont terminées : jamais sur un compte
d'images. Pool plein : un tampon propre, comme avant.

| Mesure (un lancement de 300 s) | Sans pool | Avec pool |
|---|---|---|
| Tampons d'envoi créés | 52 785 en 23 752 ms | 58 922 en 1 559 ms |
| Coût par tampon | 0,450 ms | **0,026 ms** |
| Temps total des passes de textures | 64 650 ms | 48 629 ms |
| Mémoire du pool | — | 13 morceaux, 104 Mo, 136 réemplois, 0 refus |

Filet contre la corruption (`VICTORY_UPLOAD_POISON=1`) : chaque plage est remplie d'un motif
à la fin de son bail, et vérifiée intacte avant réemploi ; une texture lue trop tard
s'afficherait en magenta. Résultat : 5 tests unitaires ; 0 octet écrit après un bail sur 197
réemplois (22 h 36 et 22 h 52) ; **0,01 % de pixels magenta** sur 240 s d'images enregistrées
(`p0-pool-poison`), contre 27 % quand on en injecte exprès (témoin `a0-magenta`).

### Les trois axes, six lancements de la même série (version normale, 300 s)

| Réglage | Heure | Médiane | 95 % | 99 % | Images > 0,8 s | Dues sous 200 à | Textures vivantes à 60 / 120 / 240 s |
|---|---|---|---|---|---|---|---|
| Défaut, sans pool | 22 h 30 | 457 ms | 692 | 1 163 | 43 | 114 s | 24 358 / 20 967 / 20 687 |
| Défaut, sans pool | 23 h 05 | 448 ms | 612 | 1 030 | 35 | 138 s | 21 923 / 19 655 / 20 754 |
| Pool | 22 h 42 | 439 ms | 573 | 970 | 34 | 129 s | 22 708 / 20 556 / 21 135 |
| Pool | 23 h 11 | 437 ms | 534 | 854 | 26 | 130 s | 21 509 / 19 977 / 20 731 |
| Pool, sans rattrapage | 22 h 58 | 444 ms | 536 | **699** | **17** | 121 s | 21 078 / 20 965 / 20 909 |
| Pool, sans rattrapage | 23 h 17 | 449 ms | 573 | 906 | **15** | 143 s | 19 113 / 18 779 / 21 059 |

Deux lancements par réglage, pas trois : les écarts de 99e centile se recouvrent (854–970
contre 699–906), le nombre d'images longues, lui, se sépare nettement (35–43, 26–34, 15–17).
La médiane ne bouge pas : c'est la carte qui la fait (310 à 330 ms ce soir).

**Ce que coûte l'absence de rattrapage** (versions diagnostic, 22 h 52 et 23 h 26) : 28 % de
textures vivantes en moins à 45 s de monde (10 964 contre 15 316), 8 % en moins à 90 s
(18 280 contre 19 877), autant ensuite. Image de contrôle en fin de lancement
(`p0-sans-rattrapage`, image 2800) : murs, affiches, lit et cheveux habillés. Les plans du
début diffèrent d'un lancement à l'autre et ne se comparent pas à l'œil.

### Fenêtres de mémoire du jeu plus fines (un essai, 23 h 33)

`VICTORY_WINDOW_KB=1024` et `VICTORY_WINDOW_TAILS=1`, deux réglages déjà présents : 2 577
fenêtres pour 3 541 Mo au plus, contre 908 à 1 012 fenêtres pour 3 824 à 4 260 Mo. Gain de
10 à 15 % seulement, mémoire partagée au même niveau (6,8 Go), temps d'image inchangé, carte
gardée. Pas retenu par défaut : le manque de mémoire partagée reste ouvert (BLOCKERS 18).

Retenu : pool par défaut ; `rapide` et `maximum` sans rattrapage (`VICTORY_TEXTURE_GATE_S=0`
dans le lanceur, `=60` pour revenir). Reste le vrai correctif des images longues : sortir la
création des copies de textures (0,5 ms chacune, 21 à 25 s par lancement) du fil qui
enregistre l'image.

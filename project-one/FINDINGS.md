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

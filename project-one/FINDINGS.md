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

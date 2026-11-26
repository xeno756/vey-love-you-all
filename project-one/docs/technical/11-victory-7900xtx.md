# 11 — Le fork VIctory-7900XTX : ce qui s'applique ici et ce qui ne s'applique pas

Question du 25/11 au soir : peut-on utiliser `github.com/avatarmanz/VIctory-7900XTX` ?
Dépôt lu et cloné hors de ce dépôt (`..\references\VIctory-7900XTX`, remote git `x7900`).

## 1. Ce qu'est ce fork

| Fait | Valeur |
|---|---|
| Base | le même fork de fxceless que nous, ancêtre commun `a803c87` |
| Taille | 393 commits depuis cet ancêtre, environ 80 000 lignes |
| Licence | GPL-3.0-or-later, la nôtre : on peut en reprendre le code en citant la source |
| Machine visée | i9-12900K, 64 Go de RAM, Radeon RX 7900 XTX 24 Go (RDNA3), 1440p |
| Résultat annoncé | image médiane de 280 ms puis 46 à 65 ms sur leur machine, profil Series X |

## 2. Pourquoi on ne peut pas le prendre tel quel

| Obstacle | Chez eux | Ici |
|---|---|---|
| Programmes mesh | dessinés en vrais mesh shaders | la RX 5600 XT (RDNA1) n'en a pas : nous les exécutons en calcul plus un vertex shader |
| Mémoire privée du processus | 40 à 46 Go (64 Go de RAM recommandés) | 15,8 Go de RAM |
| Fusion complète | — | 17 fichiers en conflit avec la branche stable, 46 avec la branche fusionnée (ScarlettPort) |

Leurs plus gros gains (enregistrements de descripteurs laissés aux dessins, 280 → 124 ms ;
listes traduites par huit fils, `VICTORY_THIN` ; soumissions jointes) touchent le cœur du
chemin de dessin, que ScarlettPort et nous avons aussi réécrit. Les reprendre demande de les
réécrire sur notre code, pas de les fusionner.

## 3. Ce qui a été repris (branche `project-one/scarlettport`)

| Commit ici | Source | Ce que ça corrige |
|---|---|---|
| `08058ad` | `1c8dd31` | Les pixel shaders lisent les autres pixels de leur quad par des opérations de quad : les lectures de vague ignorent les pixels d'aide (0 sur AMD), d'où des trous dans les ombres et des arêtes en pointillés. **Inactif par défaut ici** (section 4) |
| `7b23442` | `c5a2f8d` | Corrections d'instructions : littéraux 64 bits, bornes exactes des formats normalisés, décalages des accès FLAT, formes rares. **Retiré** (`663d05c`, section 5) |
| `a14e6dc` | `77b82c9` | Les indices de coins des primitives mesh font 9 bits, le drapeau d'arête au-dessus : lu sur 10 bits, un triangle marqué était éliminé. **Retiré** (`df2da97`, section 5) ; le test du fork qui le vérifie est marqué ignoré |

## 4. Le plantage du pilote que la première reprise a provoqué

Après ces reprises, `preparer-graphiques` s'arrêtait net, accès mémoire interdit
(code -1073741819), toujours au même enregistrement (indice 12 363 du cache de pipelines du
jeu). Une ligne de journal par enregistrement (`VICTORY_PRECOMPILE_EACH=1`) l'a nommé : le
pixel shader `4a2b54a75b9abb7c` (un éclairage de matériaux à paillettes, « glints ») avec le
vertex shader `05e958b19269d596`.

Traduit seul (`victory translate … --validate --pipelines`, avec `VICTORY_TRANSLATE_ONLY`) :
la traduction et la validation DXIL passent, c'est la création du pipeline qui plante. Le
journal d'événements de Windows nomme le module fautif : `amdxc64.dll`, le compilateur de
shaders du pilote AMD (32.0.21045.29003). Le DXIL de ce pixel shader contient 71 `quadOp` et
19 `quadReadLaneAt`, venus de `08058ad`.

`VICTORY_QUAD_READS` (dans la clé du cache de traductions, commit `83c3e60`) choisit la forme
de ces lectures :

| Valeur | Forme | Avec un vertex shader de remplacement (outil) | Dans le pipeline du jeu (précompilation) |
|---|---|---|---|
| `op` | celle du fork : `quadOp` (en x, en y, en diagonale) | **plantage du pilote** | **plantage du pilote** |
| `lane` | les quatre pixels du quad lus par `quadReadLaneAt` à indice constant, puis le bon choisi | créé (seul et avec son vrai vertex shader) | **plantage du pilote** |
| `0` (par défaut) | lectures de vague, comme avant la reprise (pixels d'aide lus à 0) | créé | **créé** |

Un plantage du pilote ferme le jeu dès qu'il fait ce pipeline : les lectures de vague restent
donc la règle sur cette carte, et la correction du fork n'y est pas active. Le test du fork
qui dessine au bord d'un triangle sur la vraie carte passe avec `lane`.

Après ce changement, et encore après les retraits de la section 5 : `preparer` 25 563
traductions, 0 refusée ; `preparer-graphiques` 25 204 pipelines faits, **0 échec** (403
refusés le matin, avant l'étage barycentrique).

## 5. Mesures en jeu (25/11, 18 h 30 à 19 h 30)

Profil `rapide`, banc `project-one\banc.ps1`. Référence : lancement de 17 h 33 sans les
reprises (médiane 318 ms, 95 % sous 430, 99 % sous 537, carte gardée).

| Heure | Version | Ce qui est repris | Monde mesuré | Médiane | 95 % sous | 99 % sous | Carte |
|---|---|---|---|---|---|---|---|
| 18 h 30 | normale | les trois (`08058ad` inactif) | moins d'une minute | — | — | — | **perdue** 71 s après le lancement : défaut de page hors de tout objet vivant |
| 18 h 41 | normale | les trois | 242 s | 332 ms | 398 | 515 | gardée |
| 18 h 48 | diagnostic, captures | les trois | 182 s | 343 ms | 434 | 839 | gardée ; têtes en mosaïque bleue |
| 19 h 07 | diagnostic, captures | `7b23442` seul actif | 13 s | — | — | — | **perdue** 73 s après l'arrivée dans le monde : défaut de page dans un tampon de travail de liste déjà libéré |
| 19 h 19 | normale | aucune (retirées) | 242 s | 325 ms | 580 | 1 843 | gardée ; 19 images de plus de 0,8 s entre 67 et 106 s de monde, côté processeur, sans pipeline en cours |
| 19 h 25 | diagnostic, captures | aucune | 121 s | 367 ms | 663 | 1 284 | gardée ; mosaïques bleues sur les murs et la tête |

Ce que ces lancements établissent, et ce qu'ils n'établissent pas :

- **Vitesse** : aucun gain mesurable. Les écarts de médiane restent sous 5 %, dans le bruit
  d'un lancement à l'autre.
- **Image** : les mosaïques bleues ne viennent pas des reprises. Le lancement de 19 h 25,
  sans elles, en montre autant ou plus, alors que celui de 17 h 12, avec le même code et les
  mêmes réglages de copies sur la carte, montrait une tête juste. L'image varie donc d'un
  lancement à l'autre avec le même code (cause ouverte, BLOCKERS 15).
- **Carte perdue** : deux pertes sur quatre lancements avec `7b23442`, aucune sur les deux
  lancements sans lui ce soir, ni sur la dizaine de lancements de l'après-midi. C'est un
  indice, pas une preuve : les deux pertes ont des signatures différentes, et la perte
  intermittente du matin n'a jamais été expliquée. Les deux reprises actives restent retirées
  tant que chacune n'a pas été mesurée seule, sur plusieurs lancements.

## 6. Ce qui reste à prendre, par ordre d'intérêt pour cette machine

| Source | Ce que c'est | Pourquoi pas encore |
|---|---|---|
| `08058ad` avec `VICTORY_QUAD_READS=lane` | les pixels d'aide lus | plante le compilateur du pilote sur au moins un pipeline du jeu ; à réessayer avec un autre pilote, ou en ne le donnant qu'aux shaders qui n'en souffrent pas |
| `7b23442`, puis `a14e6dc` | corrections d'instructions, indices sur 9 bits | à remesurer chacune seule, plusieurs lancements de suite (section 5) |
| `3bab23c` (`VICTORY_SHOW_AFTER`) | une texture n'est montrée aux shaders qu'après son envoi : leur perte de carte toutes les 16 minutes | nos deux pertes de ce soir tombent pendant l'arrivée massive des textures du monde ; à essayer en premier si la perte revient sans les reprises |
| `17c4717` | copies de tampons en Kio sur la carte | conflits dans la structure de leur `host.rs` ; nos copies de 1 Mio et plus sont déjà sur la carte (`a867de6`) |
| leurs mesures « pas la peine » | profil Series S, textures plus petites, seconde file de dessin, `VICTORY_SHADER_OPT=127` | mesurées sur leur carte : rien à gagner, inutile de les refaire |

# Agent 4 — Shaders, écran noir et remplacement des mesh shaders

Recherche en lecture seule, le 24/11/2026, sur le fork au commit `7b0f736`. Les journaux
analysés viennent du VIctory d'origine (avant le fork) sur cette machine.

## 1. Les causes d'écran noir, une par une

Dans les images du monde, 10 à 204 dessins passaient par image, contre 495 à 3 298 sautés.

| Cause | Où c'est décidé et écrit | Part sur cette carte |
|---|---|---|
| Shader non traduit | `pipeline.rs:153-166` (une fois par shader) | les mesh shaders ne sont même pas traduits ici, le refus vient avant ; ailleurs ~3,7 % d'échecs ; dans les essais : 60 calculs, 4 sommets et 2 pixels non traduits, soit 66 à 1 675 objets sautés par image |
| Pipeline refusé par la carte | `pipeline/host.rs:359-360` (absence de mesh shaders), autres refus `:338-345, 466, 494, 563, 656` | **cause principale** : 487 à 3 532 objets par image, 45 à 65 % des objets sautés. Presque toute la géométrie du monde est en mesh shaders (8 214 contre 1 633 vertex shaders, qui ne servent qu'aux effets et à la voxelisation). La taille des vagues n'est pas en cause : la carte fait 64 nativement. |
| Pipeline en cours de création | budget de 500 ms par image (`make.rs:96-107`) | 409 à 3 748 par image les ~30 premières secondes, temporaire |
| Dessin sauté pour une autre raison | `draw.rs:905, 984, 1045, 1129, 1195`, indirects `:2265-2440`, `DispatchMeshX` indirect `list.rs:1605-1619` | 64 à 128 par image (« indirect kind 0: prepare failed ») ; les textures attendent quand la mémoire vidéo dépasse 90 % du budget |
| Synchronisation / perte de la carte | barrières en `list.rs:750-758` | aucun signe qu'elle cause le noir ; en revanche 2 896 ms de travail par image puis réinitialisation, et 620 refus de mémoire puis blocage |

## 2. Comment le fork traduit les mesh shaders de la console

- **Lecture** : `native.rs:273-291, 641-658`, puis groupe à une dimension (`lower.rs:1420-1460`).
- **Prologue** (`lower.rs:3994-4060`) : triangles seulement, 128 fils au maximum, 256 sorties
  au maximum ; chaque voie élimine ses propres triangles.
- **Sorties** : la voie n écrit le sommet n et le triangle n. Chaque triangle est codé sur
  3 × 10 bits, plus un bit « éliminé » (`lower.rs:10249-10380`).
- **Dessins** :
  - directs : `DispatchMesh(x,1,1)` (`draw.rs:1801-1825`) ;
  - indirects : bornés (`indirect.rs:84-100`) puis `ExecuteIndirect` ;
  - pas d'amplification shaders.
- **Programmes du jeu** : les 8 214 mesh shaders ont tous la même forme. Il n'y a donc qu'un
  seul cas à traiter :
  - 128 × 1 × 1 fils, 128 sommets et 128 triangles, triangles uniquement ;
  - pas de charge utile, aucune donnée par triangle ;
  - 4 à 71 valeurs par sommet (médiane ~27) ;
  - 1 563 utilisent des distances de découpe, 53 dépassent 32 Kio de sorties.

## 3. Conception du remplacement

L'état au 24/11/2026 est décrit en section 3 bis.

- **Le mesh shader devient un calcul** :
  - il écrit, pour chaque groupe, 128 sommets puis 128 mots de triangle dans un tampon ;
  - les triangles inutilisés sont marqués « éliminé » et les sommets inutilisés sont
    empoisonnés ;
  - environ 14 Ko par groupe en moyenne, 37 Ko au maximum.
- **Un vertex shader généré** (sur le modèle de `rect.rs`) relit ces triangles et ces sommets
  puis écrit les mêmes sorties : le pixel shader du jeu reste inchangé. Un triangle éliminé
  devient trois fois le même point, hors de l'écran.
- **Pipelines** : un pipeline de calcul et un pipeline graphique sans format d'entrée ; la
  liaison des valeurs racines est séparée entre les deux (`draw.rs:850+`).
- **Dessins directs** : calcul, barrière, puis `DrawInstanced(groupes × 384)`.
- **Dessins indirects** : un nouveau type d'enregistrement dans le shader qui borne les
  arguments, puis deux `ExecuteIndirect`.
- **Mémoire** : un tampon par liste, 32 Mo par défaut, environ 256 Mo en vol.
- **Coût estimé** :
  - 0,3 à 0,9 Go écrits par image ;
  - deux vidages du pipeline par dessin à mesh shaders, soit environ 25 à 200 ms par image
    pour 1 200 à 3 500 dessins ;
  - regrouper les dessins consécutifs viendra plus tard.
- **Taille** : environ 1 100 lignes et 400 de tests, en 6 étapes, chacune vérifiable sans le
  jeu :
  1. variante calcul de l'émetteur, vérifiée sur WARP ;
  2. générateur de vertex shader ;
  3. comparaison pixel à pixel avec le chemin mesh shaders sur WARP (`VICTORY_CONSOLE_DXC`
     requis) ;
  4. branchement derrière `VICTORY_MESH_COMPUTE=1` ;
  5. dessins indirects ;
  6. regroupement.

## 3 bis. Ce qui est écrit (24/11/2026)

| Étape | État | Où |
|---|---|---|
| 1. Variante calcul de l'émetteur | **faite**, testée sur la RX 5600 XT (le calcul n'a pas besoin de mesh shaders) ; 8 762 des 8 765 programmes mesh du jeu se traduisent ainsi et la carte accepte leurs pipelines (commande `prepare --mesh-pipelines`) | `victory-shader/src/emit/mesh_compute.rs`, `lower.rs` |
| 2. Générateur de vertex shader | **fait**, validé et testé sur la carte (un enregistrement dessiné, triangle éliminé sauté) | `mesh_compute.rs` |
| 3. Comparaison pixel à pixel avec le vrai chemin mesh sur WARP | **pas faite** (demande `VICTORY_CONSOLE_DXC`) | — |
| 4. Branchement | **fait et vu en jeu** (444 programmes traduits, 693 pipelines, 150 à 250 dessins directs par image), automatique sur une carte sans mesh shaders ; `VICTORY_MESH_COMPUTE=1` le force, `=0` le coupe | `victory-gpu/src/pipeline/host.rs`, `draw.rs`, `host.rs` |
| 5. Dessins indirects | **faits pour un enregistrement par appel** (le cas du jeu d'après le code du fork ; 2 000 à 2 260 dessins par image), pas encore essayés en jeu ; les appels à plusieurs enregistrements ou avec compteur restent sautés | `draw.rs` (`indirect_mesh_as_compute`), `indirect.rs` (`mesh_args`) |
| 6. Regroupement des dessins | mesuré, écarté : il ferait gagner quelques millisecondes par image | `victory-gpu/src/bench.rs` |

Fonctionnement du branchement :

- **Pipelines.** Chaque pipeline mesh du jeu en donne deux. Le premier est un pipeline de
  calcul : le programme mesh traduit en calcul, avec le tampon d'enregistrements en UAV
  racine `u252`. Le second est un pipeline graphique sans format d'entrée : le vertex shader
  généré, qui lit les enregistrements en SRV racine `t252`, le pixel shader du jeu et l'état
  graphique du jeu, en liste de triangles. Les deux sont rangés dans la bibliothèque de
  pipelines sous des noms à eux.
- **Liaisons.** Le calcul lit les valeurs racines graphiques que le jeu a posées pour son
  dessin : tables, constantes, vues racines, mémoire par adresse. Il les pose avec les appels
  de calcul. Le numéro du premier groupe de la tranche est une valeur calculée par l'hôte
  (`USER_MESH_GROUP_BASE`), donc le jeu voit ses propres numéros de groupe.
- **Tranches.** Le tampon d'enregistrements est unique pour toutes les listes : 32 Mio
  (`VICTORY_MESH_RECORDS_MB`), créé au premier dessin mesh. La carte exécute les listes l'une
  après l'autre, et chaque dessin écrit ses enregistrements puis les relit avant le suivant.
  Les programmes du jeu font environ 14 Ko par groupe, donc environ 2 200 groupes par tranche.
  Les tranches suivantes ne changent que le pipeline et le numéro du premier groupe.
- **Dessins indirects.** Le nombre de groupes n'est connu que de la carte. Une petite passe de
  calcul le lit, le borne au nombre de groupes que le tampon contient, et écrit les arguments
  d'un calcul et d'un dessin, exécutés l'un après l'autre. Un dessin qui demande plus de groupes
  que le tampon n'en contient est tronqué.
- **Barrières.** Le tampon passe d'UAV à lecture avant le dessin, puis revient en UAV. Ces
  barrières font les deux vidages du pipeline par tranche prévus par la conception.
- **Coût mesuré** (banc d'essai sur la RX 5600 XT) : 2 à 10 µs par dessin mesh pour le
  schéma calcul, barrière, dessin, barrière, soit 2 à 5 ms par image. Le regroupement des
  dessins (étape 6) ferait gagner quelques millisecondes : il n'est pas écrit.

Ce qui n'est **pas vérifié** :

- l'ensemble sur un vrai programme du jeu : aucun test ne passe un conteneur de la console de
  bout en bout ;
- le rendu en jeu ;
- le coût réel par image.

Les tests couvrent le calcul (enregistrements écrits sur la carte), le vertex shader (dessin
sur la carte), le tampon unique et la passe de conversion des dessins indirects (sur la carte).
Ils ne couvrent ni les liaisons ni les tranches dans `draw.rs`.

## 4. Verdict

Ce remplacement est **nécessaire**. Sans lui, aucune géométrie du monde n'apparaît sur
cette carte. Il **ne suffit pas** : RAM (~24 Go en jeu chez l'auteur), mémoire vidéo de 6 Go,
délai de 2 s de Windows, traductions encore manquantes, ombres en ray tracing absentes.

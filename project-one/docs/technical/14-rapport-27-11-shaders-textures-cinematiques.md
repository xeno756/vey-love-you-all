# 14 — Rapport du 27/11 : shaders préparés, attentes de textures, cinématiques

Date : 27/11/2026
Machine : Ryzen 7 5700 (8 cœurs, 16 fils), 15,8 Go de RAM, Radeon RX 5600 XT 6 Go (RDNA1, sans mesh shaders).
Arbre utilisé : `victory-scarlettport` (pas `VIctory-main`).
Version : binaire `target\release\victory.exe` du 27/11 03:44.

Sources des chiffres :

- `victory-logs\20261127-040848-…personnages.err.log` et `.images.csv` (1 200 images de monde) ;
- `victory-logs\20261127-031005-…personnages.err.log` (84 images de cinématique, captures `cineA-01…16`) ;
- `victory-logs\20261127-040412-…env.json` (résumé du lanceur) ;
- `%LOCALAPPDATA%\victory\cache-scarlettport` (cache de shaders et bibliothèque de pipelines) ;
- `victory-logs\captures\planche-cineA.png`.

Aucun lancement du jeu n'a été fait pour ce rapport.

---

## 1. Actions exécutées

| Commande | Résultat | Effet sur les fichiers |
|---|---|---|
| `victory.exe translate game\shaders.rpf --list` | 26 503 conteneurs (614 doublons), **26 084 programmes traduits sur 26 084** ; DXIL : 14 637 vertex/pixel, 2 682 calcul, **8 765 mesh** ; 0 échec | aucun (lecture seule) |
| `victory.exe prepare --compute-pipelines --mesh-pipelines` | **25 563 shaders traduits, 0 non traduisible** ; **10 498 pipelines dans la bibliothèque, 0 refusé par la carte** ; 30,8 s (828,8 shaders/s) | aucun : `host-pipelines.bin` (6,34 Mo) et `early-pipelines.txt` (2 490 entrées) datent déjà du 03:42 |
| Analyse des journaux | scripts dans `work\` : `scan.py`, `prof.py`, `csv.py`→`frame_csv.py`, `why.py`, `forms.py`, `cine.py`, `ahead.py`, `block.py`, `refusals.py` | nouveaux fichiers dans `work\` |
| `prepare.txt`, `sc-translate.txt` | sorties complètes conservées | nouveaux fichiers dans `work\` |

Aucune ligne du runtime n'a été modifiée.

---

## 2. Ce qui a été observé

### 2.1 Les shaders sont tous traduits et tous les pipelines sont en cache

Dernier lancement (04:08), lignes du 09:09:29 :

```
host pipelines all made made=0 loaded=3535 started_early=2453
shaders_translated: 0    pipe_wait_ms: 0    pipelines_refuses: 0
```

- 3 535 pipelines chargés depuis la bibliothèque, 2 453 démarrés pendant le chargement, **0 fabriqué pendant le jeu** ;
- le résumé du lanceur de 04:04 donne `shaders_traduits: 0`, `attente_pipelines_ms: 0`, `pipelines_refuses: 0` ;
- la préparation de ce jour confirme la bibliothèque complète : **10 498 pipelines, 0 refusé**.

Conséquence factuelle : il n'y a plus de compilation de shaders à attendre pendant une partie. Les ralentissements mesurés ne viennent pas de là.

### 2.2 Le remplacement des mesh shaders est actif

- `crates\victory-gpu\src\pipeline\host.rs:496` : `VICTORY_MESH_COMPUTE` ; par défaut, `mesh_as_compute()` est vrai quand la carte n'a pas de mesh shaders.
- Les journaux de plantage montrent des pipelines `[mesh …] as compute`, et `prepare` a créé les 8 765 programmes mesh sous cette forme sans un seul refus.
- Le monde est donc dessiné : 2 876 à 3 060 dessins par image dans les images de monde et de cinématique.

### 2.3 Images de monde (1 200 images, `draws > 1000`)

| Mesure | Médiane | 95ᵉ centile | Maximum |
|---|---:|---:|---:|
| image | 170,8 ms (5,9 i/s) | 638,9 ms | 6 569,8 ms |
| processeur (préparation) | 97,9 ms | 273,8 ms | 2 597,0 ms |
| carte (quand chronométrée, 296 images) | 64 % d'occupation | — | — |
| recopie vers le jeu | 13,0 ms | 92,9 ms | 351,6 ms |
| dessins par image | 2 876 | 5 684 | 7 339 |
| dessins écartés | 264 | 2 096 | 10 893 |
| textures en attente | 6 942 | 31 150 | 61 182 |
| pool de textures détenu | 1 531 Mo | 1 724 Mo | 1 996 Mo |
| mémoire carte | 3 453 Mo | 3 534 Mo | 3 751 Mo |
| budget carte | 5 335 Mo | — | — |
| mémoire partagée carte | 6 280 Mo | 6 444 Mo | 7 177 Mo |
| défauts de page | 590 | 35 952 | 1 444 560 |

Le budget carte (5 335 Mo) est tenu, mais **6,3 Go de mémoire partagée** sont utilisés en plus : la carte de 6 Go travaille au-delà de sa mémoire locale.

### 2.4 Images de cinématique (84 images, captures `cineA-01…16`)

| Mesure | Médiane | Maximum |
|---|---:|---:|
| image | 223,0 ms (4,5 i/s) | 1 039,1 ms |
| processeur | 128,7 ms | 329,7 ms |
| copies de tampons du jeu | 48,1 ms / 106,6 Mo | 270,9 ms / 330,9 Mo |
| textures | 14,1 ms / 33,6 Mo | 67,5 ms / 85,9 Mo |
| carte | 82,8 ms | 136,1 ms |
| recopie vers le jeu | 80,7 ms | 106,1 ms |
| dessins | 3 060 | 3 812 |
| dessins écartés | 281 | 3 305 |

La recopie vers le jeu est **80,7 ms par image** en cinématique, contre 13,0 ms dans le monde : c'est le poste propre à la cinématique.

La planche `planche-cineA.png` montre, dans l'ordre :

- image 2 : écran presque vide (bleu-gris), sous-titres affichés ;
- image 4 : deux personnages, le visage de l'homme est noir ;
- image 5 : visage de femme éclairé et lisible ;
- images 6 à 16 : décor et personnages rendus, certaines silhouettes sombres.

Ces captures datent du lancement de 03:10, celui dont les 84 images sont comptées ci-dessus.

### 2.5 « Attentes de textures » : ce que compte la file

- `tex_waiting` = emplacements de texture placés dans le tas, dont la mémoire du jeu n'est pas encore entièrement mappée ou envoyée ;
- médiane 6 942, 95ᵉ centile 31 150 ;
- la file avance par un travail borné par image : `VICTORY_TEXTURE_SPAN_MS=400` (profil `performance` depuis le 27/11 03:10) ;
- une texture n'attend pas un shader : son descripteur existe déjà, c'est sa mémoire qui manque.

Extrait du même lancement, bloc de 5 s :

```
texture pass, last 5 s: 5 passes took 1061 ms; 2 host textures made in 179 ms;
109 staging buffers made in 752 ms
staging pool: 94624 leases, 7 chunks (56 MB), 0 asks refused
```

109 tampons d'envoi créés en 752 ms, soit 6,9 ms par tampon, alors que le pool les recycle.

### 2.6 Le poste le plus lourd : la synchronisation des descripteurs

Bloc de 5 s à 0,8 image/s (09:15) :

```
descriptor records the title copied into its heaps as bytes: 156 changed heap pages read,
319488 records looked at, 46573 made again, 751.6 ms
pages spread over threads (VICTORY_SYNC_THREADS): 137
the submitting thread waited for it after preparing (VICTORY_SYNC_AHEAD): 1081.4 ms
```

- 46 573 vues d'hôte recréées par image, **751,6 ms** de travail, réparti sur 4 fils ;
- le fil qui soumet attend **1 081 ms** ce travail avant d'envoyer ;
- les 32 octets d'un enregistrement expliquent le volume : 319 488 enregistrements / 156 pages = 2 048 par page de 4 Kio.

Pendant ce temps, la carte est vide : `nothing of ours queued 1113.0 ms (the card about 9% busy)`.

Références du projet sur ce réglage :

- `crates\victory-gpu\src\descriptor.rs:835` : 1 fil = 93 ms par image, 4 fils = 45 ms (essais du 26/11 21:58 et 22:11) ;
- 4 fils ont perdu la carte à 14 min avec `relais:3`, tenu 15 min avec `relais:2` ;
- `VICTORY_SLOT_LOCKS` est actif par défaut, donc `VICTORY_SYNC_THREADS` vaut 4 par défaut.

### 2.7 Carte et agrandisseur d'image

Le lancement de 04:08 applique trois hooks :

```
hook applied what="upscaler: always the title's own"
hook applied what="render scale: own upscaler minimum"
hook applied what="render scale: own upscaler maximum = minimum"
title resolution levers (VICTORY_RENDER_SCALE, VICTORY_OWN_UPSCALER) applied=3
```

- le jeu calcule donc en **2/3 de 1280×720**, soit 853×480, et agrandit lui-même ;
- `VICTORY_RENDER_SCALE` permet 0,5 (640×360) ou 0,4 (512×288) ;
- DLSS n'est pas exécutable sur cette carte : il demande les cœurs Tensor et le runtime NGX de NVIDIA ; la carte est AMD ;
- FSR est l'équivalent AMD, mais aucun passage FSR n'est branché dans le runtime ; l'agrandisseur du titre est déjà utilisé.

### 2.8 Xbox Series S : échec mémoire

Lancement `series-s performance` du 26/11 20:14 :

- 47 s de monde, **0 image** produite ;
- fin sur `0x8007000E` (`E_OUTOFMEMORY`) répété sur des tampons partagés de 4 Mio, puis `no host list for a state prelude`.

Le profil Series S réduit la mémoire demandée, mais ce lancement ne produit aucune image.

### 2.9 Dessins écartés

Raisons relevées dans les journaux de diagnostic (somme des compteurs par image, 25/11) :

| Raison | Compteur cumulé |
|---|---:|
| une étape du constructeur de structure de rayons, désactivée | 1 092 692 |
| ombrage des impacts de rayons, laissé de côté | 619 995 |
| un shader de calcul que la liste fournie laisse de côté | 270 859 |
| type indirect 10 : dispatch mesh indirect d'un pipeline mesh exécuté en calcul | 57 557 |
| en cours de fabrication | 31 843 |
| paires de shaders refusées | ~1 300 |

Les trois premiers postes correspondent à des fonctions désactivées par les profils, pas à des échecs de traduction.

---

## 3. Ce qui a changé

1. **Cache de shaders et bibliothèque de pipelines** : vérifiés complets et laissés en l'état.
   `prepare` n'a réécrit aucun fichier (mêmes horodatages, mêmes tailles).
2. **Aucune modification du code** du runtime, du lanceur ou de la configuration.
3. **Outils d'analyse** ajoutés dans `work\` (lecture des journaux, des CSV, des raisons d'abandon).
4. **Rapport** : ce document.

---

## 4. Ce qui devrait changer

Par ordre décroissant de gain mesuré ou estimé.

### 4.1 Descripteurs : interner les vues par contenu d'enregistrement

Fait mesuré : 46 573 vues recréées par image, 751,6 ms, dont 1 081 ms d'attente du fil de soumission.

Mécanisme proposé :

- calculer une empreinte des 32 octets d'un enregistrement ;
- si la même empreinte a déjà une vue d'hôte vivante, recopier cette vue par `CopyDescriptorsSimple` au lieu d'appeler `Create*View` ;
- ne recréer que les empreintes nouvelles.

Vérification : le compteur `made again` doit baisser, `SAME_VIEWS` doit augmenter, les captures doivent rester identiques, et `crashes.log` ne doit plus contenir de perte de carte.

Risque : la durée de vie des ressources visées par la vue recopiée ; une vue recopiée ne doit pas survivre à sa ressource.

### 4.2 Mémoire des textures

Faits : 6,3 Go de mémoire partagée pour 3,45 Go sur la carte ; 109 tampons d'envoi créés en 752 ms ; 6 942 à 31 150 textures en attente.

Leviers déjà présents :

- `textures:4` (un niveau de mip de plus laissé de côté) ;
- `echelle:0.5` (640×360 agrandi en 1280×720) ;
- `VICTORY_TEXTURE_SPAN_MS` plus haut (plus de textures par image, moins d'images/s) ;
- `VICTORY_TEXTURE_SMALLEST` pour borner la taille des plus petites.

`VICTORY_TEXTURE_THREADS` existe mais est désactivé par défaut depuis le 26/11 21:30 : tous les lancements qui perdaient la carte à une adresse basse l'avaient.

### 4.3 Cinématiques

Faits : 4,5 i/s, 223 ms par image, dont 80,7 ms de recopie vers le jeu ; visage noir sur l'image 4 de `planche-cineA.png` ; écran presque vide sur l'image 2.

Réglages déjà écrits pour ce cas :

- `VICTORY_KEEP_INDIRECT=1` : garde l'éclairage indirect ; sans lui, les visages sont noirs en cinématique (note du 26/11 19:15) ;
- `personnages` / `VICTORY_DETAIL_SHADERS=…\details-personnages.txt` et `VICTORY_DETAIL_SKIP=1` : un visage de 1024 texels n'est plus montré en 128 ;
- `attentes-textures` (`VICTORY_STAGING_PRECISE=1`) : la moitié « textures » des attentes précises ;
- `VICTORY_TEXTURE_SPAN_MS=400` : déjà dans `performance` depuis le 27/11 03:10 pour les visages de la cinématique d'ouverture ;
- `flou-et-halos` est gardé par défaut depuis le 27/11 03:10 : il porte l'exposition de la cinématique.

Le coût de `VICTORY_KEEP_INDIRECT=1` n'est pas mesuré dans les journaux disponibles.

### 4.4 Choisir le profil mesuré

`lancer.cmd` documente :

- `performance` : 6,43 i/s, médiane 153 ms, 300 s sans perte (13 h 14, avec `attentes-textures`) ;
- `performance vitesse:2` : « ce que les shaders écrivent n'est plus recopié vers la mémoire du jeu » ; le lancement de 04:08 avec ce réglage donne 6,2 i/s de médiane et des pointes à 762 ms (90ᵉ centile).

### 4.5 Series S

Le profil réduit la mémoire, mais le lancement du 26/11 20:14 s'arrête sur `0x8007000E` après 47 s de monde, sans image. Tant que cette erreur n'est pas traitée, le profil ne peut pas servir de levier mémoire.

---

## 5. Solutions : apportées et proposées

| Solution | État | Preuve ou gain | Risque |
|---|---|---|---|
| Préparation complète des shaders et pipelines | **faite** | 25 563 shaders, 10 498 pipelines, 0 refus, 30,8 s | aucun |
| Vérification du cache au lancement | **faite** | `made=0`, `shaders_translated=0`, `pipe_wait_ms=0` | aucun |
| Analyses chiffrées des journaux | **faites** | tableaux §2.3 à §2.6 | aucun |
| Agrandisseur du titre forcé | **déjà actif** | 3 hooks appliqués, rendu 2/3 | aucun |
| Interning des vues de descripteurs | **proposée** | cible : 751,6 ms → dizaines de ms | durée de vie des vues |
| Baisse de la définition interne (`echelle:0.5`) | **proposée** | 4 fois moins de pixels pour la carte ; temps carte 82,8 ms en cinématique | flou, CPU inchangé |
| Réduction des mips (`textures:4`) | **proposée** | moins de mémoire et de tampons d'envoi | textures plus grossières |
| Garder l'éclairage indirect en cinématique | **proposée** | visages noirs sans lui (note du 26/11) | coût non mesuré |
| Traiter `0x8007000E` du profil Series S | **à faire** | 0 image sur 47 s | inconnu |

---

## 6. Vue d'ensemble

1. La chaîne des shaders est terminée : 26 084 programmes traduits, 10 498 pipelines en bibliothèque, 0 refus, et aucun travail de shader pendant le dernier lancement.
2. Le remplacement des mesh shaders fonctionne : le monde et les cinématiques sont dessinés, 2 876 à 3 060 dessins par image.
3. Le temps d'image se partage entre le processeur (97,9 ms de médiane dans le monde, 128,7 ms en cinématique), la carte (64 % d'occupation quand elle est chronométrée, 82,8 ms en cinématique) et la recopie (13,0 ms dans le monde, 80,7 ms en cinématique).
4. Le poste identifiable le plus lourd est la synchronisation des descripteurs : 46 573 vues recréées, 751,6 ms, 1 081 ms d'attente du fil de soumission, carte vide à 91 %.
5. La mémoire est le second facteur : 6,3 Go de mémoire partagée sur une carte de 6 Go, 6 942 à 31 150 textures en attente, 6,9 ms par tampon d'envoi.
6. Un agrandisseur ne réduit que le temps carte ; il ne réduit ni les descripteurs ni la recopie. DLSS n'est pas exécutable sur cette carte ; l'agrandisseur du titre est déjà utilisé.
7. Les deux leviers qui restent, dans l'ordre des mesures : interning des vues de descripteurs, puis réduction de la mémoire des textures. Les cinématiques ont en plus un coût de recopie de 80,7 ms par image qui n'apparaît pas dans le monde.

---

## 7. Reproduire

```bat
cd /d "C:\Users\fatal\Downloads\Nouveau dossier\victory-scarlettport"

rem Vérifier la couverture des shaders (lecture seule)
target\release\victory.exe translate "C:\Users\fatal\Downloads\Nouveau dossier\game\shaders.rpf" --list

rem Vérifier la bibliothèque de pipelines
target\release\victory.exe prepare --compute-pipelines --mesh-pipelines

rem Lancer le profil mesuré
project-one\lancer.cmd series-x performance

rem Variante carte uniquement
project-one\lancer.cmd series-x performance echelle:0.5
```

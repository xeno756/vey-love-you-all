# Journal des performances

Une ligne par commit qui touche la vitesse ou le chargement. « Avant » et « après » sont des
mesures faites en jeu sur la même scène (l'entrée du monde au début de la partie), par
l'utilisateur ; « non mesuré » veut dire qu'aucun lancement n'a encore eu lieu avec ce
commit. Le fichier `<journal>.images.csv` de chaque lancement donne les temps image par
image depuis le 25/11 à 11 h 20.

Machine : Ryzen 7 5700, 15,8 Go de RAM, RX 5600 XT 6 Go. Branche `project-one/scarlettport`.

## Lancements de référence

| Lancement | Réglage | Temps d'image dans le monde (relevés de 5 s) | Carte | Processeur (avant + après la carte) | Textures chargées | Fin |
|---|---|---|---|---|---|---|
| 25/11 9 h 41 | aucun (rendu 2560 × 1440) | médiane 1 050 ms, 95 % sous 1 934 ms, pire 6 034 ms | 777 ms | 183 + 90 ms | 6 596 en 7 min 35 | manque de mémoire |
| 25/11 10 h 08 | ancien `rapide` (1280 × 720, textures:2, 4 groupes d'effets) | médiane 950 ms sur 2 min 06, 460 ms les 40 dernières secondes | 221 ms | 217 + 92 ms | 67 915 en 2 min 06 | fermé par l'utilisateur |
| 25/11 10 h 11 | ancien `rapide` + `vitesse` | médiane 1 247 ms sur 55 s (trop court pour comparer) | 142 ms | 258 + 170 ms | 36 061 en 55 s | fermé ; image blanchie |
| 25/11 10 h 13 | ancien `maximum` (960 × 540) | le monde n'est jamais atteint : 15 dessins par image pendant 2 min | — | — | 0 | fermé ; resté sur l'écran des licences |

Ces centiles sont ceux de moyennes sur 5 secondes : les centiles image par image n'existent
que depuis le commit `7a95d07`.

## Lancements mesurés image par image (25/11, depuis 11 h 48)

Lancés par moi : profil `rapide` (1280 × 720, textures:2, aucun effet laissé de côté), A
pressé jusqu'au monde, puis la cinématique d'ouverture sans rien toucher, fermeture par le
numéro de processus. Les temps sont ceux du fichier `.images.csv`, monde uniquement.

| Heure | Version | Règle des textures | Durée dans le monde | Temps d'image : médiane / 95 % / 99 % | 60 dernières images : médiane, processeur + carte + recopie | Textures chargées | Blocages de 30 images sans envoi | Fin |
|---|---|---|---|---|---|---|---|---|
| 11 h 57 | normale | réserve (`dfca70d`) | 99 s | 724 / 2 702 / 7 076 ms | 720 ms, 235 + 167 + 157 | 66 018 | 0 | **carte perdue** |
| 12 h 01 | diagnostic, temps par commande | réserve | 241 s | 558 / 1 837 / 3 201 ms | 543 ms, 146 + 316 + 67 | 76 660 | 0 | fermé par le test |
| 12 h 06 | normale | ancienne (`VICTORY_STREAM_POOL=0`) | 242 s | 455 / 1 901 / 2 496 ms | 510 ms, 142 + 281 + 67 | 53 832 | **3** (88 images de suite au plus) | fermé par le test |
| 12 h 11 | diagnostic, couche de validation | réserve | 150 s | 853 / 2 799 / 4 098 ms | — | 74 892 | 0 | fermé par le test ; aucun appel invalide signalé |
| 12 h 15 | diagnostic, traces | réserve | 242 s | 507 / 1 814 / 2 524 ms | 497 ms, 151 + 267 + 68 | 83 494 | 0 | fermé par le test |
| 12 h 21 | normale, traces | réserve | 302 s | 466 / 1 502 / 2 704 ms | 456 ms, 129 + 257 + 66 | 84 553 | 0 | fermé par le test |

| 12 h 28 | normale, traces | réserve | 63 s | 1 319 / 3 053 / 10 019 ms | — | 38 204 | 0 | **carte perdue** : défaut de page à une adresse hors de tout objet |
| 12 h 33 | normale | réserve, détail fixe (`VICTORY_STREAM_PRESSURE=0`) | 301 s | 510 / 1 276 / 2 015 ms | 519 ms, 141 + 300 + 67 | 57 161 | 0 | fermé par le test |
| 13 h 58 | normale, correctif du transfert | réserve | 302 s | 504 / 1 132 / 3 307 ms | 503 ms, 157 + 255 + 85 | 36 442 (réserve pleine : budget de la carte tombé à 3,9 Go, Edge et Discord rouverts) | 0 | fermé par le test |
| 14 h 05 | normale, les deux correctifs, détail ajouté au réglage | réserve | 302 s | 506 / 1 713 / 2 542 ms | 515 ms, 160 + 266 + 84 | 66 333 | 0 | fermé par le test |
| 14 h 12 | normale, les deux correctifs | réserve | 301 s | 668 / 1 995 / 4 388 ms | 656 ms, 145 + 419 + 82 (la carte était aussi prise par d'autres programmes) | 48 752 | 0 | fermé par le test |
| 14 h 21 | normale, les deux correctifs, **réglages bas du jeu** (`qualite-s`) | réserve | 303 s | 442 / 1 636 / 2 564 ms | 440 ms, 128 + 240 + 66 | 69 725 | 0 | fermé par le test |

Ce que ces lancements établissent :

- **La réserve des textures fait ce qu'elle doit** : aucun blocage en cinq lancements, là où
  l'ancienne règle se bloque trois fois en quatre minutes, et 56 % de textures en plus au
  même instant (84 183 contre 53 832 après 240 s de monde).
- **Elle ne change pas le temps d'image** : 456 à 510 ms en régime établi dans les deux cas,
  soit environ 2 images par seconde.
- **La perte de carte de la version normale** : 11 h 48, 11 h 57 et 12 h 28 (2 fois sur 3
  dans la même configuration, plus le lancement avec passage automatique des menus), jamais
  avec un niveau de détail fixe ni dans la version diagnostic. Le journal de 12 h 28 donne un
  défaut de page à une adresse qui n'appartient à aucun objet. Cause probable, corrigée par
  `f69b10f` : l'état de transfert d'une texture servait pour une copie refaite à une autre
  taille, et une copie pouvait être libérée entre le moment où on la regarde et celui où on
  copie dedans. Depuis : 4 lancements de 300 s sans panne (13 h 58, 14 h 05, 14 h 12,
  14 h 21). Aucun des deux cas n'a été vu se présenter dans ces lancements (le journal
  l'écrirait) : la cause est probable, pas démontrée.
- **Les réglages bas du jeu** (`9967d6a`, option `qualite-s`) : 440 ms par image en régime
  établi contre 506 à 515, 14,9 Go de mémoire demandée contre environ 18,5, mémoire de la
  carte 3,3 Go contre 3,6, mémoire partagée 5,7 Go contre 6,4. Un lancement de chaque
  sorte ; l'image n'est pas encore vérifiée.
- **Où part le temps de la carte en 1280 × 720** (12 h 01, temps par commande) : 189 ms par
  image en tout, dont 136 ms dans 4 549 dessins indirects de programmes mesh, 32 ms dans
  3 860 calculs, 13 ms dans 388 dessins mesh directs. Les personnages (12 ms en 200
  commandes) et les ombres en cascade (environ 22 ms en 1 200 commandes) sont en tête.
- **Dessins sans pipeline, par image** (12 h 01) : 631 calculs d'ombrage de lancer de rayons
  et 347 étapes du constructeur de structures, laissés de côté exprès ; 34 dessins de
  cheveux en mèches et 4 de peau refusés faute de barycentriques ; 4 d'un filtre de halo.

## L'image, vérifiée sur les images que le runtime enregistre lui-même (25/11, 14 h 25 à 14 h 44)

Même version, même scène (la cellule et sa fenêtre sur la cour), version diagnostic, une image
enregistrée toutes les 20. Les images sont dans `..\victory-captures` (`essai-x`, `essai-s`,
`essai-x2`), hors du dépôt.

| Réglage | Ce que l'image montre | Temps d'image en régime établi |
|---|---|---|
| `rapide` d'avant (réglages normaux du jeu) | taches roses, bleues et vertes sur les murs et les personnages, voile blanc sur les plans de la cour ; murs présents | 537 ms (170 + 267 + 92) |
| avec les réglages bas du jeu (`qualite-s`) | éclairage propre, sans tache ; **les murs de la cellule manquent** (on voit le ciel à leur place) | 376 ms (130 + 172 + 63) |
| réglages normaux, éclairage indirect laissé de côté (`sans:eclairage-indirect`) | plus de taches, murs présents, intérieurs plus sombres, traînées autour de la fenêtre | 398 ms (132 + 177 + 68) |

Cause des taches : l'éclairage indirect du jeu s'appuie sur son lancer de rayons, que le
runtime n'exécute pas (631 calculs d'ombrage et 347 étapes du constructeur de structures
laissés de côté par image). Calculé sans lui, il donne ces couleurs. Le profil `rapide` le
laisse de côté depuis `528d504`.

## Le profil `rapide` d'aujourd'hui (25/11, 14 h 50 et 14 h 57, version normale)

1280 × 720, textures:2, éclairage indirect laissé de côté. Sur les images de même contenu
(4 000 à 5 500 dessins, après la première minute de monde) :

| Réglage | Images mesurées | Temps d'image médian | Processeur + carte + recopie |
|---|---|---|---|
| `rapide` | 601 | **335 ms** (3,0 images par seconde) | 113 + 168 + 53 |
| `rapide` avec l'envoi en avance (`VICTORY_RELAY=1`) | 115 | 303 ms (3,3 images par seconde) | 110 + 184 + 61, dont 52 ms en même temps |

Les deux lancements ont tenu 300 s sans panne ni blocage, 87 000 à 88 000 textures chargées
en 120 s. Le relais gagne environ 10 % : il reste une option.

### Les réglages de vitesse du côté du processeur (25/11, 15 h 04 à 15 h 28)

| Réglage ajouté à `rapide` | Temps d'image sur les mêmes plans | Image | Fin |
|---|---|---|---|
| `vitesse` (contrôles plus légers, copies évitées) | 381 ms contre 409 (version diagnostic) | présente, avec des traînées | fermé par le test |
| `vitesse:2` tel qu'il était (recopie limitée à ce que le jeu relit) | 196 ms contre 335 (version normale) | **noire** : aucun tampon relu n'était reconnu, rien n'était recopié | fermé par le test |
| `vitesse:2`, reconnaissance corrigée (`2d8ba5a`) | 247 ms contre 409 (version diagnostic) | **noire** encore : 81 tampons reconnus ne suffisent pas | fermé par le test |
| `vitesse:2` avec les zones jusqu'à 1 Mio recopiées | environ 250 ms contre 400 (version diagnostic, 12 images comparables) | présente | **carte perdue après 72 s** : défaut de page dans des copies de mémoire du jeu déjà libérées |

`vitesse:2` vaut donc environ 40 % de temps d'image en moins, mais n'est pas utilisable : il
reste une copie de mémoire du jeu libérée alors que la carte s'en sert encore.

### Pipelines refusés : 0 (25/11, 16 h 12 et 16 h 32)

La carte n'a pas de barycentriques (`BarycentricsSupported` lu faux au démarrage). Depuis
`12e7c2b`, un pixel shader qui les lit est traduit une seconde fois pour les lire dans des
entrées ordinaires, et le pipeline reçoit un geometry shader généré qui les écrit.

| Mesure, profil `rapide` | Avant | Après |
|---|---|---|
| Pipelines refusés par lancement | 3 à 8 | **0** (4 faits avec l'étage) |
| Dessins laissés de côté pour refus, par image | 42 (34 cheveux, 4 peau, 4 halo) | 0 |
| Temps d'image médian, mêmes plans | 335 ms | 334 ms |
| 99 % des images sous | 1 138 ms | 839 ms |
| Carte perdue | non | non |

Les cheveux sont maintenant dessinés, mais mal : une masse pâle et morcelée autour de la tête
(images enregistrées par le runtime, `..\victory-captures\essai-bary`). Ce qui vient de
l'étage et ce qui vient de l'éclairage des cheveux du jeu, qui s'appuie sur son lancer de
rayons, n'est pas établi. `VICTORY_BARYCENTRIC_GS=0` rend l'état d'avant (tête sans cheveux).

### Le contrat de 20 images par seconde

Pas tenu : 3 images par seconde en régime établi. Ce qui l'empêche, mesuré sur le lancement
de 16 h 32 (moyennes par image) : processeur **113 ms**, carte **168 ms**, recopie **55 ms**.
Le processeur et la recopie font à eux seuls 168 ms, plus de trois fois les 50 ms d'une image
à 20 par seconde : même une carte infiniment rapide laisserait 6 images par seconde. Baisser
encore la charge de la carte (résolution, effets) ne peut donc pas tenir le contrat ; c'est
le travail du processeur qu'il faut réduire d'abord (préparation des soumissions, copies de
la mémoire du jeu, recopie de ce que les shaders écrivent : `vitesse:2` en retire 40 %, mais
perd encore la carte, BLOCKERS 11).

Banc d'essai versionné : `project-one\banc.ps1` (lance, appuie jusqu'au monde, mesure 300 s,
résume : médiane, 95e et 99e centiles, images de plus de 50 et de 200 ms, pipelines refusés,
carte perdue).

### Le processeur : comparaisons confiées à la carte (25/11, 16 h 50 à 17 h 40)

Avant chaque image, le processeur prenait une copie de 380 Mo de tampons du jeu pour trouver
ensuite ce que les shaders y avaient écrit : 349 Mo de ces copies font entre 1 et 16 Mio, sous
le seuil à partir duquel le runtime les met sur la carte (16 Mio sur une carte de 6 Go).

| Réglage ajouté à `rapide` (version normale, mêmes plans) | Médiane | 95 % sous | 99 % sous | Processeur + carte + recopie | Image (captures du runtime) |
|---|---|---|---|---|---|
| aucun (référence de 16 h 32) | 334 ms | 411 | 839 | 110 + 168 + 54 | cheveux faux |
| copies de 1 Mio et plus sur la carte (`a867de6`, désormais dans `rapide`) | **318 ms** | 430 | **537** | **78** + 207 + **34** | **cheveux justes**, quelques taches bleues sur un mur |
| les mêmes + relais | 293 ms | 354 | 495 | 81 + 208 + 56 | **taches et mosaïques partout** |
| les mêmes + relais + fenêtres et fusion d'indirects, attentes allégées | 265 ms | 300 | 377 | 71 + 189 + 50 | **taches et mosaïques partout** |
| les mêmes sans relais + fenêtres, fusion, attentes allégées (diagnostic) | 350 ms contre 354 | — | — | — | plus de taches bleues |

Retenu : les copies sur la carte, sans relais ni réglages de synchronisation, qui abîment
l'image. La carte devient le premier poste (207 ms par image).

Ce que la carte fait de ces 207 ms (mesure par commande du 25/11, 12 h 01) : 136 ms dans
4 549 dessins indirects de programmes mesh par image, soit 30 µs chacun ; chaque dessin passe
par 4 à 6 synchronisations de la carte (bornage des arguments, conversion en arguments de
calcul et de dessin, calcul des enregistrements, dessin). C'est le prochain chantier : grouper
ces dessins pour ne plus synchroniser la carte à chacun.

### Reprises du fork VIctory-7900XTX (25/11, 18 h 30 à 19 h 30)

Trois commits repris, six lancements mesurés ; tout est dans
[docs/technical/11-victory-7900xtx.md](docs/technical/11-victory-7900xtx.md). En bref :
aucun gain de vitesse mesurable ; deux pertes de carte sur quatre lancements avec les
reprises, aucune sur deux sans elles ; les mosaïques bleues de l'image varient d'un lancement
à l'autre avec le même code et ne viennent pas des reprises. Les deux reprises actives sont
retirées. La lecture des pixels voisins par opérations de quad fait planter le compilateur de
shaders du pilote AMD sur un pipeline du jeu : elle reste coupée par défaut.

### La pâte bleue (25/11, 20 h à 21 h)

Trouvée et corrigée : [FINDINGS.md](FINDINGS.md), chantier A. L'éclairage par tuiles lisait
6 textures de lumière rebondie que rien n'écrit (passes laissées de côté ou demandant le
lancer de rayons) : elles gardaient ce que la mémoire du jeu contenait. Remplies d'une
constante, la pâte disparaît (0,0 % de pixels bleus sur deux lancements, contre jusqu'à
23,7 % avant), sans coût mesurable.

### P0 : pertes de carte, pool de tampons d'envoi, arrivée des textures (25/11, 22 h à 23 h 45)

Mesures et preuves : [FINDINGS.md](FINDINGS.md), chantier P0. Série faite avec d'autres
programmes sur la carte (310 à 330 ms de carte par image au lieu de 200) : ses chiffres ne se
comparent qu'entre eux. Médiane inchangée (437 à 457 ms ce soir-là) : ces correctifs visent
les pertes de carte et les images longues, pas le temps de carte.

### Itération 4 : banc propre, mise bout à bout, relais (26/11, 0 h à 1 h 30)

Mesures par lancement : [FINDINGS.md](FINDINGS.md), itération 4. État des lots :
[LOT_LEDGER.md](LOT_LEDGER.md). Série faite avec Discord ouvert (banc propre refusé) : ses
chiffres ne se comparent qu'entre eux. Le banc écrit maintenant un fichier `.env.json` par
lancement (programmes sur la carte, pilote, horloge, température et activité de la carte).

Chemin parcouru sur cette scène : 1 050 ms par image le matin en 2560 × 1440, 494 ms avec le
premier `rapide`, 335 ms maintenant.

### Mission 10 à 15 images par seconde, cycle 1 (26/11, matin)

Audit et premier levier : [FINDINGS.md](FINDINGS.md), cycle 1. Départ : 2,68 images par
seconde, 95 % des images sous 418 ms (`rapide`, 26/11 à 1 h 00). Objectifs : 10 images par
seconde avec 95 % sous 100 ms, puis 15 avec 95 % sous 66,7 ms.

Je ne lance plus le jeu (DECISIONS 22) : les mesures viennent des séries du banc que
l'utilisateur lance.

**Cycle 1 mesuré (7 h 50 et 7 h 57, banc propre, 300 s de monde, l'utilisateur jouait) :**
`performance` donne 4,68 et 4,45 images par seconde, médiane 201 et 217 ms, 95 % sous 265
et 266 ms, sans perte de carte ni manque de mémoire. `rapide` en banc propre : 327 à 337 ms.

**Cycle 2 préparé (8 h)** : le fil qui soumet attend la carte 12,8 à 14 fois par image
(135 à 154 ms). Trois réglages de ScarlettPort ont un mot dans le lanceur (`precis`,
`relais:2`, `fenetres`) ; aucun n'est mesuré.

### Mes lancements (26/11, à partir de 8 h 54)

L'utilisateur m'a de nouveau permis de lancer le jeu (DECISIONS 29). Banc propre, 300 s de
monde, cinématique d'ouverture sans personne aux commandes : les plans sont les mêmes d'un
lancement à l'autre.

| Heure | Réglage | Images par seconde | Médiane / 95 % / 99 % | Processeur + carte + recopie | Attentes | Fin |
|---|---|---|---|---|---|---|
| 8 h 54 | `performance` (version complète, `c2ed234`) | **8,77** | 110 / 162 / 254 ms | 50 + 51 + 28 | 2,8 par image, 51 ms | 300 s, carte tenue |
| 9 h 07 | + `VICTORY_MESH_WINDOW_ARGS=1` (`38da7cf`) | — | — | — | — | **carte perdue à 45 s** (adresse d'aucun objet) |
| 9 h 11 | + le même, liaisons de calcul refaites à chaque dessin | 8,63 | 110 / 171 / 337 ms | 48 + 53 + 26 | 3 par image, 51 ms | **carte perdue à 230 s** (même signature) ; réglage retiré (`5572a82`) |
| 9 h 20 | `performance` + `VICTORY_WINDOW_PAST_BUFFER_WRITES=1` (`9e52aa5`), **Discord ouvert** (banc propre refusé) | — | — | — | 0 attente avant la panne | **carte perdue à 27 s** (même signature : adresse d'aucun objet, aucune liste du jeu en cours) |
| 9 h 23 | la même chose **sans `fenetres`** (`VICTORY_INDIRECT_WINDOW=0`), Discord ouvert | — | — | — | 0,4 à 1,5 attente par image | **carte perdue à 35 s** : une liste du jeu arrêtée à une barrière (commande 2 349 sur 3 906) ; `fenetres` n'est donc pas la cause |
| 9 h 28 | la même chose, **version diagnostic** avec `traces`, Discord (610 Mo) et OBS ouverts | — | — | — | — | **carte perdue à environ 40 s** ; aucune liste du jeu en cours, défaut à une adresse d'aucun objet : le rapport ne nomme pas de commande fautive |
| 9 h 38 | B : `performance-sur vitesse:3` (brouillard **gardé**, sans `fenetres`), Discord et OBS ouverts | 7,6 sur la dernière minute | — | — | — | **carte perdue à environ 2 min** : le brouillard n'est pas la cause |
| 9 h 42 | C : `performance-sur fenetres sans:volumetrie-collecte` + textures 80 ms + fenêtres qui passent les tampons, **sans `vitesse:3`**, Discord et OBS ouverts | 5,33 | 181 / 210 / 403 ms | 78 + 96 + 100 | 4,3 par image, 95 ms | **240 s, carte tenue** : `vitesse:3` est la cause |
| 9 h 48 | D : C + 256 listes en vol (`VICTORY_LATE_WAIT_LISTS=256`), Discord et OBS ouverts | 5,99 | 164 / 236 / 396 ms | 76 + 67 + 112 | 1,7 par image, 67 ms | 240 s, carte tenue ; aucun dessin refusé faute d'emplacements ; la carte est occupée 81 % du temps |
| 10 h 02 | E : D + la comparaison de la carte par groupe (`a191a66`), Discord et OBS ouverts | **6,31** | 151 / 223 / 397 ms | 82 + 52 + 88 | 1,1 par image, 55 ms | 240 s, carte tenue ; carte occupée 73 % (81 % avant), et pourtant 3 160 dessins par image au lieu de 2 590 : devenu `performance` (`532831c`) |
| 10 h 09 | `performance` (`532831c`), **version diagnostic** avec captures, Discord (720 Mo) ouvert | — | — | — | — | **carte perdue avant le monde** (même signature) : la panne n'est donc pas seulement celle de `vitesse:3` ; les captures n'ont montré que le noir du début |
| 10 h 12 et 10 h 16 | F : `performance` tel que commité (`532831c`), série de 2 | 7,86 et 8,07 | 121 et 115 / 201 et 197 ms | 65–67 + 36–38 + 69–72 | 1,1 par image, 34–37 ms | **carte perdue les deux fois** (vers 2 min de monde) : sans `vitesse:3` aussi |
| 10 h 21 et 10 h 24 | G : le même **sans** les fenêtres qui passent (`VICTORY_WINDOW_PAST_BUFFER_WRITES=0`) | 3,27 et 6,66 | 289 et 142 ms | — | — | **perdue au premier, tenu 300 s au second** : ce réglage n'est pas la cause seul |
| 10 h 31 et 10 h 38 | H : `performance` avec le **relais au niveau 1** et **sans attentes précises** | **6,13 et 5,98** | 158 et 161 / **190 et 194** / 291 et 385 ms | 65–66 + 91–92 + 31–32 | — | **300 s tenu les deux fois** : devenu `performance` (`6fd9440`) |

À comparer : le même réglage sans la correction des objets libérés avait perdu la carte à
90 s (8 h 25). Un lancement ne prouve pas la stabilité : il en faut d'autres.

**Correction (10 h 30).** Le paragraphe suivant accusait `vitesse:3` à tort. Sur toute la
journée, le facteur commun des pertes est le **relais au niveau 2 avec les attentes
précises** (`relais:2 precis`, entrés dans les profils à 8 h 13) : 10 pertes sur 15
lancements avec eux, 1 sur 12 sans eux (la nuit et le matin, relais au niveau 1 ; la seule
perte était un essai volontairement sous-dimensionné de la réserve d'envoi). `vitesse:3`
rend la perte plus probable, sans en être la seule cause. L'amont avait laissé ces deux
réglages éteints pour des défauts de ce genre. Mesuré ensuite (série H, 10 h 31 et 10 h 38) : `performance`
avec le relais au niveau 1 et sans attentes précises tient 300 s deux fois sur deux, à 6,0 et
6,1 images par seconde. C'est la version `performance` depuis `6fd9440`.

**Ce que ces pannes ont en commun (9 h 30).** Sur la journée, 6 lancements sur 7 avec
`vitesse:3` ont perdu la carte (8 h 25, 9 h 07, 9 h 11, 9 h 20, 9 h 23, 9 h 28 ; seul 8 h 54
a tenu), et 0 sur 3 sans lui (7 h 50, 7 h 57, 8 h 13). Le brouillard laissé de côté
(`sans:volumetrie-collecte`) est arrivé dans le même lancement que `vitesse:3` (8 h 25) : les
deux sont suspects. Les deux mènent au même genre de faute : une valeur jamais écrite (une
passe du brouillard absente) ou jamais recopiée vers le jeu (`vitesse:2` et `3` ne recopient
plus ce que les shaders écrivent, sauf pour les tampons que le jeu relit) peut servir
d'adresse à un shader qui lit la mémoire du jeu par adresse : le défaut tombe alors
n'importe où, sur rien. Je les essaie séparément.

En régime établi (de 2 à 5 minutes de monde) : 8,6 à 9,7 images par seconde, 103 à 117 ms par
image, la carte au travail 62 à 69 % du temps. Le chargement (première minute) tourne à 3,6
à 6,6 images par seconde au lieu de 2 à 3, et dure deux fois plus longtemps (26 000
textures vivantes à 50 s au lieu de 25 s).

## Commits

| Date | Commit | Lot | Réglage qui l'annule | Avant | Après | Risque |
|---|---|---|---|---|---|---|
| 26/11 | `ab1bec0` | mission, cycle 4, stabilité | `VICTORY_RETIRE_MS=0` (l'attente d'avant : deux images) | carte perdue après 90 s à 8–9 images par seconde (8 h 25), défaut de page dans des objets libérés | voir « Mes lancements » plus haut | les objets libérés vivent une seconde de plus : jusqu'à quelques centaines de Mo de plus pendant le chargement |
| 26/11 | `63caa72`, `c2ed234` | mission, cycle 4, chargement | `VICTORY_TEXTURE_SPAN_MS=250` | première minute de monde : jusqu'à 250 ms par image à créer des textures, 2 à 3 images par seconde | voir « Mes lancements » plus haut | les textures arrivent plus tard |
| 26/11 | lanceur, `vitesse:3` (`87d13b1`) | mission, cycle 3 | ne pas donner `vitesse:2` ni `vitesse:3` | avec `precis relais:2` : 570 morceaux (36 Mo) relus de la carte par image, 60 à 104 ms d'attente du fil de recopie ; 5,0 à 5,2 images par seconde sur les plans de référence | avec `vitesse:3 fenetres sans:volumetrie-collecte` : **7,8 à 8,6 images par seconde** sur les mêmes plans (113 à 121 ms), 5 à 23 morceaux relus | carte perdue après 90 s (BLOCKERS 28) ; image non vérifiée sur capture |
| 26/11 | `19f3e56` | mission, cycle 2 | ne pas donner `precis`, `relais:2`, `fenetres` | `performance` : 12,8 à 14 attentes de la carte par image, 135 à 154 ms sur 200 ; 4,45–4,68 images par seconde | `precis relais:2` : **4,97 images par seconde**, médiane 187 ms ; 4 attentes par image mais encore 119 ms ; 0 alerte, 0 perte sur 1 lancement. `fenetres` : non mesuré | image non vérifiée sur capture |
| 26/11 | `19f3e56` | mission, image | `sans:transparence` le redonne | haut du corps du personnage proche absent (capture d'écran de 7 h 54) | **non vérifié** : `performance` ne laisse plus le groupe `transparence` de côté | quelques calculs de plus par image, coût non mesuré |
| 26/11 | `6eaf67c` | mission, carte | ne pas donner `sans-ombres` ni `performance` (sans `VICTORY_SKIP_DRAW_SHADERS`, rien ne change) | 4 549 dessins indirects mesh par image, dont au moins 1 300 pour les cartes d'ombres (25 à 30 ms de carte) | 1 810 à 2 220 dessins écartés par image ; mesuré avec le profil entier (ligne `a409572`), pas seul | plus d'ombres portées : des rectangles de soleil traversent les murs |
| 26/11 | `aad682c` | mission, mesure | — (colonnes en fin de fichier) | ni défauts de page ni temps noyau par image | quatre colonnes de plus (45) ; `performance` : 18 700 à 19 200 défauts de page par image, 94–98 ms noyau et 393–397 ms utilisateur (tous les fils) | deux appels système de plus par image quand le fichier est écrit |
| 26/11 | `a409572` | mission, profil | ne pas donner `performance` | `rapide` : 327 à 337 ms de médiane en banc propre (3 images par seconde) | **4,68 et 4,45 images par seconde**, médiane 201 et 217 ms, 95 % sous 265 et 266 ; processeur 56 + carte 149–161 + recopie 33–35 ; 0 perte de carte sur 2 | image : haut du corps du personnage proche absent (groupe `transparence`, retiré du profil ensuite), soleil à travers les murs |
| 25/11 | `432cb67` | profils | ne pas donner `rapide` | 2560 × 1440 : carte 777 ms, 6 596 textures en 7 min 35, arrêt par manque de mémoire | 1280 × 720 + textures:2 : carte 221 ms, 67 915 textures en 2 min 06, carte dans son budget (4,2 Go sur 4,7) | image moins nette ; taches colorées vues à ce lancement, cause non établie |
| 25/11 | `dfca70d` | 1, streaming | `VICTORY_STREAM_POOL=0` | sans profil : aucune texture envoyée tant que la carte dépasse 90 % de son budget | **non mesuré** ; hors jeu : la réserve des textures vaut au moins 512 Mio, et 16 textures par image sont faites même pleine (tests) | textures plus grossières quand la réserve se remplit ; pas de retour au détail fin ensuite |
| 25/11 | `7a95d07` | 0, mesure | ne pas donner `VICTORY_FRAME_CSV` | pas de mesure image par image | une ligne par image, centiles sur 300 images dans la barre de titre | une demande de mémoire à la carte par image quand le fichier est écrit |
| 25/11 | `ee2d30b` | 3, diagnostic | — | « pipeline refusé : paramètre incorrect » | la capacité manquante est nommée (barycentriques) | aucun |
| 25/11 | `ee55801` | 2, diagnostic | `VICTORY_D3D12_DRED` vide avant le lancement | une carte perdue ne disait rien dans la version normale | adresse fautive dans le journal de toute version ; même temps d'image (456 ms avec, 455 à 510 sans) | aucun vu |
| 25/11 | `f69b10f` | 1, perte de carte | — | carte perdue 2 fois sur 3 (63 et 99 s de monde) | 4 lancements de 300 s sans panne | la cause n'est pas démontrée |
| 25/11 | `dd14e44` | 1, streaming | `VICTORY_STREAM_PRESSURE=0` | carte à 3,9 Go de budget : 36 442 textures en 300 s, réserve pleine | 66 333 textures en 300 s | textures jusqu'à 3 niveaux plus grossières que demandé, jamais sous 128 pixels |
| 25/11 | `9967d6a` | 4, réglages bas du jeu | ne pas donner `qualite-s` | 506 à 515 ms par image, 18,5 Go demandés | 440 ms, 14,9 Go | image non vérifiée ; une seule mesure |
| 25/11 | `528d504` | image et 6, charge de la carte | `sans:` autre chose, ou `series-x` sans profil | taches sur toute l'image ; carte 267 ms par image | plus de taches ; carte 177 ms ; image 398 ms contre 537 | intérieurs plus sombres, traînées autour des sources de lumière |
| 25/11 | `2d8ba5a` | 5, recopie | ne pas donner `vitesse:2` | image noire avec `VICTORY_COPY_BACK_ONLY_READBACK` | image présente avec `VICTORY_COPY_BACK_SMALL_KB=1024`, environ 250 ms contre 400 | carte perdue une fois après 72 s : expérimental |
| 25/11 | `12e7c2b` | 3, pipelines | `VICTORY_BARYCENTRIC_GS=0` | 3 à 8 pipelines refusés, 42 dessins par image perdus | 0 refusé, aucun dessin perdu pour refus ; même temps d'image | cheveux dessinés mais mal rendus |
| 26/11 | `7ee9224` | L, pool | `VICTORY_STAGING_POOL=0`, `VICTORY_RETIRE_FRAMES=0` | une plage du pool rendue avec les texels d'avant ; un tampon à part détruit sans attendre (1 perte de carte sur 1 lancement) | plage rendue à zéro ; 0 perte sur 1 ; poison : 35 144 baux réemployés, 0 écriture tardive | un essai de chaque côté pour la perte de carte |
| 26/11 | option `relais` (existante) | G0, mise bout à bout | ne pas la passer | médiane 361, 376, 380 ms | 312, 322, 325 ms ; série « Discord ouvert » | pas par défaut : murs absents une fois sur deux, avec ou sans (BLOCKERS 21) |
| 25/11 | `c4cd932` | P0, pertes de carte | `VICTORY_RETIRE_FRAMES=0` | 2 pertes sur 2 lancements (objets libérés encore lus) | 0 sur 10 | les objets vivent 2 images de plus (mémoire négligeable) |
| 25/11 | `c4cd932` | P0.2, pool | `VICTORY_STAGING_POOL=0` | 52 785 tampons d'envoi créés en 23,8 s par lancement | 58 922 en 1,6 s ; 26 à 34 images > 0,8 s contre 35 à 43 | une lecture trop tardive corromprait une texture : filet `VICTORY_UPLOAD_POISON=1`, rien trouvé |
| 25/11 | `3628498` | P0.6, arrivée | `VICTORY_TEXTURE_GATE_S=60` | 26 à 34 images > 0,8 s (avec pool) | 15 à 17 ; 99 % sous 699 et 906 ms | 28 % de textures en moins à 45 s de monde, 8 % à 90 s |
| 25/11 | `181cba5` | B1, mesure | — (compteurs) ; `VICTORY_TEXTURE_CATCHUP_MS` (1500) | aucune mesure du débit des pipelines ; grappe du 99e centile sans cause | pipelines : 0 ms d'attente, 0 traduction en jeu ; grappe : rattrapage des textures, 72 % en création de ressources | aucun : compteurs et réglage, défaut inchangé |
| 25/11 | lanceur (`rapide`, `maximum`) | A, image | `VICTORY_AMBIENT_FILL=off` | pâte bleue sur 4 lancements de 5 (jusqu'à 23,7 % des pixels) | 0,0 % sur 2 lancements ; médiane 327 ms contre 325 | intérieurs éclairés d'une lumière rebondie constante, pas la vraie |
| 25/11 | `83c3e60` | 3, pipelines | `VICTORY_QUAD_READS=lane` ou `op` (les rallume) | `preparer-graphiques` plante à l'enregistrement 12 363 (compilateur du pilote) | 25 204 pipelines faits, 0 échec | aucun : même traduction qu'avant la reprise `08058ad` |
| 25/11 | `df2da97`, `663d05c` | 8, non-régression | — (retraits) | 2 pertes de carte sur 4 lancements avec les reprises | 0 sur 2 lancements sans | indices de 9 bits et corrections d'instructions du fork absents |
| 25/11 | `a867de6` | 5, processeur | `VICTORY_CARD_BUFFERS_MIN_MB=16` | processeur 110 ms, recopie 54, 99 % sous 839 ms | processeur 78, recopie 34, 99 % sous 537 ; médiane 318 | la carte prend 1 Go de plus pour ces copies (les textures en ont moins) |
| 25/11 | `fd37912` | 2, figement | `sortie:960x540` le redonne | `maximum` : resté sur l'écran des licences | **non mesuré** ; `maximum` demande 1280 × 720, la taille vue en jeu | `maximum` laisse 8 groupes d'effets de côté : image possiblement fausse |

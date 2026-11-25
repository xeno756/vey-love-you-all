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

Chemin parcouru sur cette scène : 1 050 ms par image le matin en 2560 × 1440, 494 ms avec le
premier `rapide`, 335 ms maintenant.

## Commits

| Date | Commit | Lot | Réglage qui l'annule | Avant | Après | Risque |
|---|---|---|---|---|---|---|
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
| 25/11 | `fd37912` | 2, figement | `sortie:960x540` le redonne | `maximum` : resté sur l'écran des licences | **non mesuré** ; `maximum` demande 1280 × 720, la taille vue en jeu | `maximum` laisse 8 groupes d'effets de côté : image possiblement fausse |

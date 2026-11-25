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

Ce que ces lancements établissent :

- **La réserve des textures fait ce qu'elle doit** : aucun blocage en cinq lancements, là où
  l'ancienne règle se bloque trois fois en quatre minutes, et 56 % de textures en plus au
  même instant (84 183 contre 53 832 après 240 s de monde).
- **Elle ne change pas le temps d'image** : 456 à 510 ms en régime établi dans les deux cas,
  soit environ 2 images par seconde.
- **Une perte de carte intermittente existe dans la version normale** : à 11 h 48 (lancement
  avec passage automatique des menus) et à 11 h 57, pas à 12 h 21 ni avec l'ancienne règle
  à 12 h 06, et jamais dans les quatre lancements de la version diagnostic. Cause non
  trouvée ; depuis `ee55801` le journal de la version normale donne l'adresse fautive.
- **Où part le temps de la carte en 1280 × 720** (12 h 01, temps par commande) : 189 ms par
  image en tout, dont 136 ms dans 4 549 dessins indirects de programmes mesh, 32 ms dans
  3 860 calculs, 13 ms dans 388 dessins mesh directs. Les personnages (12 ms en 200
  commandes) et les ombres en cascade (environ 22 ms en 1 200 commandes) sont en tête.
- **Dessins sans pipeline, par image** (12 h 01) : 631 calculs d'ombrage de lancer de rayons
  et 347 étapes du constructeur de structures, laissés de côté exprès ; 34 dessins de
  cheveux en mèches et 4 de peau refusés faute de barycentriques ; 4 d'un filtre de halo.

## Commits

| Date | Commit | Lot | Réglage qui l'annule | Avant | Après | Risque |
|---|---|---|---|---|---|---|
| 25/11 | `432cb67` | profils | ne pas donner `rapide` | 2560 × 1440 : carte 777 ms, 6 596 textures en 7 min 35, arrêt par manque de mémoire | 1280 × 720 + textures:2 : carte 221 ms, 67 915 textures en 2 min 06, carte dans son budget (4,2 Go sur 4,7) | image moins nette ; taches colorées vues à ce lancement, cause non établie |
| 25/11 | `dfca70d` | 1, streaming | `VICTORY_STREAM_POOL=0` | sans profil : aucune texture envoyée tant que la carte dépasse 90 % de son budget | **non mesuré** ; hors jeu : la réserve des textures vaut au moins 512 Mio, et 16 textures par image sont faites même pleine (tests) | textures plus grossières quand la réserve se remplit ; pas de retour au détail fin ensuite |
| 25/11 | `7a95d07` | 0, mesure | ne pas donner `VICTORY_FRAME_CSV` | pas de mesure image par image | une ligne par image, centiles sur 300 images dans la barre de titre | une demande de mémoire à la carte par image quand le fichier est écrit |
| 25/11 | `ee2d30b` | 3, diagnostic | — | « pipeline refusé : paramètre incorrect » | la capacité manquante est nommée (barycentriques) | aucun |
| 25/11 | `fd37912` | 2, figement | `sortie:960x540` le redonne | `maximum` : resté sur l'écran des licences | **non mesuré** ; `maximum` demande 1280 × 720, la taille vue en jeu | `maximum` laisse 8 groupes d'effets de côté : image possiblement fausse |

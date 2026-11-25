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

## Commits

| Date | Commit | Lot | Réglage qui l'annule | Avant | Après | Risque |
|---|---|---|---|---|---|---|
| 25/11 | `432cb67` | profils | ne pas donner `rapide` | 2560 × 1440 : carte 777 ms, 6 596 textures en 7 min 35, arrêt par manque de mémoire | 1280 × 720 + textures:2 : carte 221 ms, 67 915 textures en 2 min 06, carte dans son budget (4,2 Go sur 4,7) | image moins nette ; taches colorées vues à ce lancement, cause non établie |
| 25/11 | `dfca70d` | 1, streaming | `VICTORY_STREAM_POOL=0` | sans profil : aucune texture envoyée tant que la carte dépasse 90 % de son budget | **non mesuré** ; hors jeu : la réserve des textures vaut au moins 512 Mio, et 16 textures par image sont faites même pleine (tests) | textures plus grossières quand la réserve se remplit ; pas de retour au détail fin ensuite |
| 25/11 | `7a95d07` | 0, mesure | ne pas donner `VICTORY_FRAME_CSV` | pas de mesure image par image | une ligne par image, centiles sur 300 images dans la barre de titre | une demande de mémoire à la carte par image quand le fichier est écrit |
| 25/11 | `ee2d30b` | 3, diagnostic | — | « pipeline refusé : paramètre incorrect » | la capacité manquante est nommée (barycentriques) | aucun |
| 25/11 | `fd37912` | 2, figement | `sortie:960x540` le redonne | `maximum` : resté sur l'écran des licences | **non mesuré** ; `maximum` demande 1280 × 720, la taille vue en jeu | `maximum` laisse 8 groupes d'effets de côté : image possiblement fausse |

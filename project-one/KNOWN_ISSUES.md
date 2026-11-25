# Problèmes connus

Classés du plus bloquant au moins bloquant, sur cette machine.

| # | Problème | Effet | Cause | Statut |
|---|---|---|---|---|
| 1 | Pas de mesh shaders sur la RX 5600 XT | le monde (cinématique, jeu) reste noir ; 45 à 65 % des objets sautés par image dans les essais | toute la géométrie du monde de PROJECT GAME est en mesh shaders (8 214 contre 1 633 vertex shaders d'effets) | remplacement écrit et **vu en jeu** pour les dessins directs (444 programmes traduits, 693 pipelines) ; hors jeu, 8 762 des 8 765 programmes mesh du jeu se traduisent en calcul et la carte accepte leurs pipelines ; dessins indirects à un enregistrement écrits, **pas encore essayés en jeu** ; coût mesuré du schéma : 2 à 5 ms par image |
| 2 | RAM : 19 à 21,6 Go demandés en mode Series X pour 15,8 Go (mesuré) | 4,5 Go seulement du processus restent en RAM, 150 000 à 650 000 défauts de page par image, images de 16 à 20 s | mémoire du jeu (13,5 Gio en Series X) + copies pour la carte | ouvert. Le mode Series S (7,8 Go pour le jeu) reste bloqué, voir 9 |
| 3 | Mémoire vidéo : 5,3 à 6,6 Go utilisés pour un budget de 4,85 Go en mode Series X (mesuré) | débordement en RAM, carte ralentie | une copie par texture utilisée, 2,7 Go de render targets en 1440p | plancher adapté ; tampon mesh unique ; **textures réduites** (`textures:2`, jusqu'à 1 465 Mo concernés), à essayer en jeu ; partage des pages à faire |
| 3 bis | La carte prend 0,4 à 2,6 s par image en mode Series X (mesuré), processeur presque inactif | environ 1 image par seconde dans le monde | environ 30 passes de calcul plein écran en 1440p, shaders traduits | **effets laissés de côté par groupe** (`sans:…`, `minimal`), à essayer en jeu ; profil par commande (`lancer.cmd series-x profil`) pour le temps de chaque passe. Le jeu calcule en 2560 × 1440 sans réglage possible : voir 11 |
| 4 | Réinitialisation de la carte (TDR) | crash en entrant dans le monde, déjà vu deux fois | lots de calcul de 2,5 à 3 s, préemption du calcul par liste seulement | rapport amélioré (durée, préemption) ; découpage à faire. Le délai TDR de Windows n'est pas modifié. |
| 5 | Ombres noires | même sur RTX 4070 | lumière et ombres en ray tracing non traduites | dans le fork, en cours |
| 6 | Premiers lancements | 650 à 4 180 dessins sautés par image pendant 4 min 30, et 500 ms d'attente par image sur les fils d'enregistrement (mesuré) | 1 200 à 1 600 pipelines à compiler, 3,5 par seconde | commande `prepare` (pipelines de calcul et moitié calcul des pipelines mesh compilés avant le lancement) ; ordre de compilation revu ; les pipelines de dessin se compilent encore en jeu la première fois |
| 7 | R11G11B10 au lieu de R9G9B9E5 (Series S) | précision un peu moindre sur cette cible | la carte refuse R9G9B9E5 en cible | accepté ; rendu non vérifié |
| 8 | Attente de mémoire de transfert | une pause jusqu'à 2 s si la mémoire partagée sature | choix de stabilité : attendre plutôt qu'échouer | à mesurer en jeu |
| 9 | Mode Series S | restait sur le premier écran du jeu (2 380 images, aucun fichier chargé ensuite) | le runtime disait au jeu qu'il utilisait presque toute sa mémoire | cette cause est corrigée mais **le blocage persiste** (lancement de 7 minutes le 25/11 : « 7 617 Mo utilisés sur 7 776 », dont 4 679 Mo de pages physiques du jeu). Cause inconnue, piste mise de côté |
| 10 | Peu d'essais en jeu | trois lancements Series X (7, 8 et 2 minutes) ont montré le remplacement des mesh shaders, les dessins indirects et la préparation ; ce qui a été écrit le 25/11 (textures réduites, effets laissés de côté) n'a pas encore tourné avec PROJECT GAME | les lancements sont faits par l'utilisateur | tests unitaires, tests sur la carte et bancs d'essai uniquement |
| 11 | Définition du jeu en mode Series X | 2560 × 1440 en interne et 3840 × 2160 présentés par défaut (mesuré sur les captures) | le jeu choisit selon le type de console | réglable depuis le 25/11 : `1080p`, `720p`, `540p`, `sortie:WxH`, `echelle:N` ([08](docs/technical/08-resolution-scarlettport-intro.md)) ; `1080p` démarre (lancement de 9 h 00), le rendu dans le monde reste à voir |
| 12 | Branche fusionnée (`meilleure.cmd`) : carte perdue à la première image du monde | fermeture 65 s après le démarrage, lancements de 8 h 59 et 9 h 00 | une erreur de la fusion : la table que 94 % des shaders lisent n'était jamais branchée (deux numéros d'emplacement identiques) | corrigé le 25/11 à 9 h 30, **pas encore essayé en jeu** ([09](docs/technical/09-plantages-du-25-11.md), section 4). La branche stable n'a pas ce défaut |

## Ce qui n'est **pas** un problème de PROJECT ONE

- **Les deux échecs `emit::rect`** : ils viennent du validateur DXIL trop ancien du SDK
  Windows. Ils passent avec `VICTORY_DXIL` (voir README).
- **L'avertissement « GamingRepair » vu avec XCL** : c'est le texte du jeu lui-même, quand on
  lui refuse le Store hors ligne. Aucune dépendance ne manque.

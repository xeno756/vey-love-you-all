# Problèmes connus

Classés du plus bloquant au moins bloquant, sur cette machine.

| # | Problème | Effet | Cause | Statut |
|---|---|---|---|---|
| 1 | Pas de mesh shaders sur la RX 5600 XT | le monde (cinématique, jeu) reste noir ; 45 à 65 % des objets sautés par image dans les essais | toute la géométrie du monde de PROJECT GAME est en mesh shaders (8 214 contre 1 633 vertex shaders d'effets) | remplacement écrit et **vu en jeu** pour les dessins directs (444 programmes traduits, 693 pipelines) ; dessins indirects à un enregistrement écrits, **pas encore essayés en jeu** ; coût mesuré du schéma : 2 à 5 ms par image |
| 2 | RAM : 19 à 21,6 Go demandés en mode Series X pour 15,8 Go (mesuré) | 4,5 Go seulement du processus restent en RAM, 150 000 à 650 000 défauts de page par image, images de 16 à 20 s | mémoire du jeu (13,5 Gio en Series X) + copies pour la carte | mode Series S débloqué (7,8 Go pour le jeu), **à vérifier en jeu** |
| 3 | Mémoire vidéo : 5,3 à 6,6 Go utilisés pour un budget de 4,85 Go en mode Series X (mesuré) | débordement en RAM, carte ralentie | une copie par texture utilisée, 2,7 Go de render targets en 1440p | plancher adapté ; tampon mesh unique ; mode Series S à essayer ; partage des pages à faire |
| 3 bis | La carte prend 0,4 à 2,6 s par image en mode Series X (mesuré), processeur presque inactif | environ 1 image par seconde dans le monde | environ 30 passes de calcul plein écran en 1440p, shaders traduits | mode Series S à essayer, puis profil par commande (`lancer.cmd series-s profil`) |
| 4 | Réinitialisation de la carte (TDR) | crash en entrant dans le monde, déjà vu deux fois | lots de calcul de 2,5 à 3 s, préemption du calcul par liste seulement | rapport amélioré (durée, préemption) ; découpage à faire. Le délai TDR de Windows n'est pas modifié. |
| 5 | Ombres noires | même sur RTX 4070 | lumière et ombres en ray tracing non traduites | dans le fork, en cours |
| 6 | Premiers lancements | 650 à 4 180 dessins sautés par image pendant 4 min 30, et 500 ms d'attente par image sur les fils d'enregistrement (mesuré) | 1 200 à 1 600 pipelines à compiler, 3,5 par seconde | commande `prepare` (pipelines de calcul compilés avant le lancement) ; ordre de compilation revu ; les pipelines de dessin se compilent encore en jeu la première fois |
| 7 | R11G11B10 au lieu de R9G9B9E5 (Series S) | précision un peu moindre sur cette cible | la carte refuse R9G9B9E5 en cible | accepté ; rendu non vérifié |
| 8 | Attente de mémoire de transfert | une pause jusqu'à 2 s si la mémoire partagée sature | choix de stabilité : attendre plutôt qu'échouer | à mesurer en jeu |
| 9 | Mode Series S | restait sur le premier écran du jeu (2 380 images, aucun fichier chargé ensuite) | le runtime disait au jeu qu'il utilisait presque toute sa mémoire | **corrigé, à vérifier en jeu** |
| 10 | Peu d'essais en jeu | un seul lancement de 7 minutes en mode Series X a montré le remplacement des mesh shaders ; rien de ce qui a été écrit ensuite (Series S, dessins indirects, préparation) n'a encore tourné avec PROJECT GAME | les lancements sont faits par l'utilisateur | tests unitaires, tests sur la carte et bancs d'essai uniquement |

## Ce qui n'est **pas** un problème de PROJECT ONE

- **Les deux échecs `emit::rect`** : ils viennent du validateur DXIL trop ancien du SDK
  Windows. Ils passent avec `VICTORY_DXIL` (voir README).
- **L'avertissement « GamingRepair » vu avec XCL** : c'est le texte du jeu lui-même, quand on
  lui refuse le Store hors ligne. Aucune dépendance ne manque.

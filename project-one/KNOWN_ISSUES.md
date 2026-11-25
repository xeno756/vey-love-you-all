# Problèmes connus

Classés du plus bloquant au moins bloquant, sur cette machine.

| # | Problème | Effet | Cause | Statut |
|---|---|---|---|---|
| 1 | Pas de mesh shaders sur la RX 5600 XT | le monde (cinématique, jeu) reste noir ; 45 à 65 % des objets sautés par image dans les essais | toute la géométrie du monde de PROJECT GAME est en mesh shaders (8 214 contre 1 633 vertex shaders d'effets) | remplacement écrit pour les dessins directs (calcul + vertex shader généré), **pas encore essayé en jeu** ; dessins indirects encore sautés ; coût : deux vidages du pipeline par tranche |
| 2 | RAM : ~24 Go utilisés en jeu chez l'auteur du fork, 15,8 Go ici | fichier d'échange intensif, manque de mémoire possible | mémoire du jeu (13,5 Gio en Series X) + copies pour la carte | mémoire de transfert bornée ; le reste n'est pas réductible sans perte |
| 3 | Mémoire vidéo : 6 Go | textures retenues près du budget, débordement en RAM | une copie par texture utilisée, render targets jamais partagés | plancher adapté ; partage des pages à faire |
| 4 | Réinitialisation de la carte (TDR) | crash en entrant dans le monde, déjà vu deux fois | lots de calcul de 2,5 à 3 s, préemption du calcul par liste seulement | rapport amélioré (durée, préemption) ; découpage à faire. Le délai TDR de Windows n'est pas modifié. |
| 5 | Ombres noires | même sur RTX 4070 | lumière et ombres en ray tracing non traduites | dans le fork, en cours |
| 6 | Premier lancement | objets sautés quelques secondes | pipelines en cours de création (attente bornée à 500 ms par image) | normal, se résorbe |
| 7 | R11G11B10 au lieu de R9G9B9E5 (Series S) | précision un peu moindre sur cette cible | la carte refuse R9G9B9E5 en cible | accepté ; rendu non vérifié |
| 8 | Attente de mémoire de transfert | une pause jusqu'à 2 s si la mémoire partagée sature | choix de stabilité : attendre plutôt qu'échouer | à mesurer en jeu |
| 9 | Mode Series S du fork | jamais lancé sur cette machine | — | à tester (moins de mémoire, 720p interne) |
| 10 | Rien de tout ça n'a tourné avec PROJECT GAME | — | consigne : pas de lancement | tests unitaires et GPU uniquement |

## Ce qui n'est **pas** un problème de PROJECT ONE

- **Les deux échecs `emit::rect`** : ils viennent du validateur DXIL trop ancien du SDK
  Windows. Ils passent avec `VICTORY_DXIL` (voir README).
- **L'avertissement « GamingRepair » vu avec XCL** : c'est le texte du jeu lui-même, quand on
  lui refuse le Store hors ligne. Aucune dépendance ne manque.

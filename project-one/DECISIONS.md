# Décisions prises par défaut

Prises le 25/11/2026 pour avancer sans poser de question. Chacune se change en le disant.

| # | Question | Décision | Raison |
|---|---|---|---|
| 1 | Qui lance PROJECT GAME ? | Jusqu'au 25/11 à 11 h 30, l'utilisateur seul. Depuis, moi aussi : l'utilisateur l'a demandé (« oui lance toi même et vérifie le tout en temps réel »). | Chaque lancement de mesure passe par `meilleure.cmd`, appuie sur A jusqu'au monde, laisse tourner la cinématique d'ouverture 300 s et ferme le jeu par son numéro de processus. Le passage automatique des menus de ScarlettPort (`VICTORY_FAST_BOOT`) n'est pas utilisé : il ouvre le menu pause. |
| 10 | Fermer d'autres programmes avant un lancement ? | Une fois, à la demande de l'utilisateur (Edge, Discord, l'outil de capture). Il les a rouverts ensuite : je ne les ferme plus. | Les mesures d'après 13 h 58 sont donc faites avec moins de mémoire de carte (budget de 3,9 à 4,1 Go au lieu de 4,7). |
| 11 | Vérifier l'image | Par les images que le runtime enregistre lui-même (version diagnostic), pas par une capture de l'écran. | Une capture de l'écran a pris une autre fenêtre que le jeu ; elle a été supprimée. |
| 2 | Quelle scène de test ? | L'entrée du monde au début de la partie (la cinématique d'ouverture), la même à chaque lancement, sans rien toucher. | C'est la seule scène que tous les lancements ont en commun. `VICTORY_FAST_BOOT=1` de ScarlettPort passe les écrans du début tout seul : à essayer pour des lancements identiques. |
| 3 | Un affichage dans l'image ? | Non : la barre de titre de la fenêtre (centiles, mémoire de la carte, textures en attente, « STREAM STALL ») et un fichier CSV par lancement. | Dessiner du texte dans l'image demande un rendu de texte que le runtime n'a pas. |
| 4 | Quelle part du budget pour les textures ? | 90 % du budget de la carte moins ce qui n'est pas des textures, 512 Mio au moins. | La règle « 80 % moins 300 Mo » aurait réduit de 730 à 512 Mo la réserve du lancement de 10 h 08, qui tenait dans son budget. Les 10 % restants font 460 à 530 Mo de marge sur cette carte. |
| 5 | `rapide` laisse-t-il des effets de côté ? | Non. | Toute dégradation voulue de l'image doit être une option. Les images à taches colorées viennent du lancement `rapide` qui en laissait quatre de côté : cause non établie, donc on les retire du profil. |
| 6 | Garde-t-on 960 × 540 ? | Non, plus aucun profil ne le demande. | Le jeu n'a pas dépassé l'écran des licences à cette taille. |
| 7 | Préréglage bas du jeu (qualité Series S, mémoire Series X) ? | Oui : option `qualite-s`, écrite et mesurée le 25/11 à 14 h 21. Elle reste une option tant que l'image n'est pas vérifiée. | Le monde se charge en 150 s sans blocage et quatre lancements de suite ont tenu 300 s : les conditions de l'ordre des lots étaient remplies. |
| 8 | Un chemin Vulkan pour les barycentriques ? | Non. | Ce runtime ne parle que D3D12. |
| 12 | Le « gouverneur » qui baisse la qualité jusqu'à tenir 20 images par seconde ? | Pas écrit. | Sa propre règle de sortie s'applique déjà : même au plus bas, le processeur et la recopie dépassent 50 ms par image (168 ms mesurées). Il ne tiendrait rien ; le travail va au processeur. |
| 13 | Un affichage dans l'image ? | Toujours la barre de titre et le CSV (décision 3). | Inchangé. |
| 9 | Ordre des commits des lots 0 et 1 | Le lot 1 avant le lot 0. | Le fichier par image lit les compteurs que le lot 1 ajoute. La mesure de départ, elle, vient des journaux des lancements déjà faits. |

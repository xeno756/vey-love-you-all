# Décisions prises par défaut

Prises le 25/11/2026 pour avancer sans poser de question. Chacune se change en le disant.

| # | Question | Décision | Raison |
|---|---|---|---|
| 1 | Qui lance PROJECT GAME ? | L'utilisateur, comme depuis le début. Le runtime n'est lancé par moi que pour préparer les shaders ou passer les tests. | La consigne « ne lance jamais le jeu » a été donnée plusieurs fois et aucun message ne l'a levée en toutes lettres. Conséquence : « re-mesurer » veut dire le lancement suivant de l'utilisateur ; ce qui peut se mesurer hors jeu l'est. |
| 2 | Quelle scène de test ? | L'entrée du monde au début de la partie (la cinématique d'ouverture), la même à chaque lancement, sans rien toucher. | C'est la seule scène que tous les lancements ont en commun. `VICTORY_FAST_BOOT=1` de ScarlettPort passe les écrans du début tout seul : à essayer pour des lancements identiques. |
| 3 | Un affichage dans l'image ? | Non : la barre de titre de la fenêtre (centiles, mémoire de la carte, textures en attente, « STREAM STALL ») et un fichier CSV par lancement. | Dessiner du texte dans l'image demande un rendu de texte que le runtime n'a pas. |
| 4 | Quelle part du budget pour les textures ? | 90 % du budget de la carte moins ce qui n'est pas des textures, 512 Mio au moins. | La règle « 80 % moins 300 Mo » aurait réduit de 730 à 512 Mo la réserve du lancement de 10 h 08, qui tenait dans son budget. Les 10 % restants font 460 à 530 Mo de marge sur cette carte. |
| 5 | `rapide` laisse-t-il des effets de côté ? | Non. | Toute dégradation voulue de l'image doit être une option. Les images à taches colorées viennent du lancement `rapide` qui en laissait quatre de côté : cause non établie, donc on les retire du profil. |
| 6 | Garde-t-on 960 × 540 ? | Non, plus aucun profil ne le demande. | Le jeu n'a pas dépassé l'écran des licences à cette taille. |
| 7 | Préréglage bas du jeu (qualité Series S, mémoire Series X) ? | Oui, c'est la voie retenue pour réduire les polygones. Pas commencée. | L'ordre des lots l'interdit tant que le monde ne se charge pas entièrement et que le démarrage n'est pas sûr, ce que seul un lancement dira. |
| 8 | Un chemin Vulkan pour les barycentriques ? | Non. | Ce runtime ne parle que D3D12. |
| 9 | Ordre des commits des lots 0 et 1 | Le lot 1 avant le lot 0. | Le fichier par image lit les compteurs que le lot 1 ajoute. La mesure de départ, elle, vient des journaux des lancements déjà faits. |

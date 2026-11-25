# Blocages

Ce qui n'a pas pu être fait ou prouvé, ce qui est livré à la place, et ce qu'il faut pour
lever le blocage. Mis à jour le 25/11/2026.

| # | Blocage | Livré à la place | Pour le lever |
|---|---|---|---|
| 1 | Aucune mesure en jeu des commits du 25/11 après 11 h. Je ne lance pas le jeu ([DECISIONS.md](DECISIONS.md), 1). | Tests hors jeu (763 réussis), capacités de la carte lues, shaders préparés. | Un lancement de `meilleure.cmd series-x rapide` ; son fichier `.images.csv` et son journal donnent la mesure. |
| 2 | En 960 × 540 le jeu reste sur l'écran des licences. La cause est dans le jeu et n'est pas trouvée : la taille d'écran annoncée ou la taille d'image imposée. | `maximum` demande 1280 × 720. | Deux lancements qui séparent les deux réglages, ou l'étude de la fonction du jeu qui attend. |
| 3 | Pas d'éviction des textures les moins utiles. Le jeu choisit ses textures par indice dans un tas de 37 000 vues : le runtime ne sait pas lesquelles un shader lit vraiment. | Les copies deviennent plus grossières quand leur réserve se remplit, 16 textures par image sont faites même pleine, et les textures que le jeu décharge sont rendues comme avant. | Un retour de la carte sur les textures lues (un compteur par vue écrit par les shaders traduits). |
| 4 | Une texture faite grossière ne redevient pas fine quand la place revient. | Rien : elle garde son niveau jusqu'à ce que le jeu la recharge. | Refaire les copies les plus grossières quand la réserve repasse sous la moitié. |
| 5 | Temps de la carte passe par passe : seulement dans la version diagnostic. | `meilleure.cmd series-x rapide profil`. | Rien à lever : c'est un lancement plus lent, à faire quand une passe est à chercher. |
| 6 | Image blanchie au lancement `rapide vitesse` de 10 h 11. Cause non établie (réglages de ScarlettPort, ou simple scène claire). | `vitesse` reste une option, hors des profils. | Un lancement `rapide` sans `vitesse` au même endroit de la cinématique. |
| 7 | Taches colorées au lancement `rapide` de 10 h 08. Trois causes possibles, aucune prouvée : les 4 groupes d'effets laissés de côté ; le calcul d'activation des rayons des sondes, laissé de côté sur AMD ; autre chose. | `rapide` ne laisse plus aucun effet de côté : le prochain lancement écarte ou confirme la première cause. | Ce lancement. Si les taches restent : une préparation puis un lancement avec `VICTORY_SHADER_OPT=on`, qui remet le calcul d'activation. |
| 8 | 4 pipelines refusés : leur pixel shader lit les barycentriques, que la carte n'a pas (mesuré : `BarycentricsSupported` faux). | Le refus nomme maintenant cette capacité. | Lot 3 : faire donner ces coordonnées par le shader de sommets généré pour les programmes mesh. Pas commencé. |

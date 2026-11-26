# Registre des lots

Un lot est FAIT, EN COURS, À FAIRE ou REFUSÉ (avec la raison). Mis à jour à chaque itération.
Les preuves sont dans [FINDINGS.md](FINDINGS.md), ce qui bloque dans [BLOCKERS.md](BLOCKERS.md),
les choix dans [DECISIONS.md](DECISIONS.md).

Dernière mise à jour : 26/11/2026, itération 4.

## Lot E — métrologie

| Lot | État | Ce qui est fait, ce qui manque |
|---|---|---|
| E1 banc propre | FAIT | `banc.ps1 -Propre` (ou `VICTORY_BENCH_CLEAN=1`) refuse de mesurer si un programme tiers tient plus de 100 Mo de la carte, et les nomme. Il ne ferme rien lui-même (DECISIONS 15). |
| E2 empreinte d'environnement | FAIT, hors CSV | Un fichier `.env.json` par lancement, à côté du CSV : programmes sur la carte, pilote, alimentation, fichier d'échange, PCIe, horloge / température / activité / puissance de la carte toutes les 5 s (capteurs AMD). L'état de bridage n'est pas exposé par cette carte. Dans un fichier à part plutôt qu'en colonnes (DECISIONS 16). |
| E3 statistique | FAIT | `-Series N`, une ligne par lancement, images de plus de 0,8 s comme mesure principale. |
| E4 collisions | FAIT | Un seul banc à la fois (verrou), refus si le jeu tourne déjà, fichier de touches propre à chaque lancement. |
| E5 cache chaud | FAIT | La série lance `preparer` et `preparer-graphiques` avant de mesurer et s'arrête s'ils échouent ; un lancement qui traduit des shaders ou attend des pipelines en jeu est marqué « cache froid ». |
| E6 colonnes en fin de CSV | FAIT | Règle tenue (41 colonnes, les 18 dernières ajoutées en fin). |

## Lot R — RAM et mémoire partagée

| Lot | État | Ce qui est fait, ce qui manque |
|---|---|---|
| R1 cartographie des fenêtres | FAIT par catégorie | 877 fenêtres, 3 604 Mo : 88 % couvrent de la mémoire du jeu en partie non chargée (FINDINGS, itération 4). Manque : le compte exact des pages touchées, qui demande un compteur dans la version diagnostic. |
| R2 matérialisation à la page | À FAIRE, tête de file | Le runtime tient déjà une table par page de 64 Kio. `VICTORY_INPLACE` écarté par mesure (10 % des octets au plus, et la mémoire reste comptée comme partagée). |
| R3 plafond et admission | À FAIRE | |
| R4 dédoublonnage, compteur de va-et-vient | À FAIRE | |
| R5 fichier d'échange et engagement | FAIT | 15,8 Go de RAM, fichier d'échange de 34 Go, limite d'engagement 49 Go : le jeu tient, mais avec 3 à 58 Mo de RAM libre il pagine. Dit dans FINDINGS. |

## Lot C — copies de textures

| Lot | État | Ce qui est fait, ce qui manque |
|---|---|---|
| C1 copies placées | À FAIRE | Carte de lecture faite : possible seulement pour les textures simples, dans des tas neufs (seule garantie de zéros), le tas lié à la vie de la ressource. |
| C2 création hors du fil d'enregistrement | À FAIRE, premier pas de C | Carte de lecture faite : la création peut partir sur un fil à part par `resource::host_copy` ; l'envoi et l'écriture de l'emplacement restent sur le fil du jeu. |
| C3 règle d'aliasing | À FAIRE | Avec C1. |
| C4 rattrapage ré-évalué | À FAIRE | Après C1 et C2. |

## Lot L — durée de vie

| Lot | État | Ce qui est fait, ce qui manque |
|---|---|---|
| L1 retrait par soumission généralisé | À FAIRE | Contournement en place (`VICTORY_RETIRE_FRAMES=2`), étendu aux tampons d'envoi créés à part (`7ee9224`). Carte de lecture faite : le compteur est à prendre à `queue::execute` ; les tampons de travail gardés en réserve sont réemployés sans aucune règle. |
| L2 magenta expliqué | FAIT | Deux sources : (a) jamais écrit, une plage du pool rendue avec le contenu d'avant, corrigé par `7ee9224` ; (c) attendu, les couleurs des illustrations de chargement. Après correction : 0,00 % dans les images du monde, 0 écriture tardive. |
| L3 couverture du poison | FAIT | Pool réduit à 24 Mo, 300 s : 35 144 baux (713 Mo) depuis des morceaux réemployés, 3 morceaux sur 3, 0 octet écrit après un bail. |

## Lot G — la médiane

| Lot | État | Ce qui est fait, ce qui manque |
|---|---|---|
| G0 sérialisation | FAIT pour la mesure et l'essai | Bout à bout à 91–98 %. Relais : médiane 312, 322, 325 ms contre 361, 376, 380 (−54 ms), image propre sur deux captures. Pas adopté par défaut (DECISIONS 19). Série « Discord ouvert », à refaire en banc propre. |
| G1 cascade par passe | À FAIRE | Carte de lecture faite : le mode `profil` compte par commande et par pipeline, pas par passe ; il faut y ajouter des repères avant nos propres passes. |
| G2 registre du contrat | FAIT | Ci-dessous, à tenir à jour. |
| G3 dessins indirects regroupés | À FAIRE | Après G1 (porte 3). |
| G4 occupation des shaders | À FAIRE | Demande RGA (lot T). |
| G5 meshlets cuits | À FAIRE | |
| G6 lancer de rayons | FAIT pour l'état | Le runtime ne l'exécute pas : 631 calculs d'ombrage et 347 étapes de construction laissés de côté par image, dit dans le journal. Aucune traversée en calcul ne tourne. |

## Lot T — outils AMD

| Lot | État | Ce qui est fait, ce qui manque |
|---|---|---|
| T1 outils officiels | À FAIRE, attend ton accord | Télécharger RGA, RGP et RMV depuis GPUOpen est un téléchargement de binaires : je ne le fais pas sans ton oui explicite. |
| T2 barycentriques par AGS | REFUSÉ pour l'instant | Sans objet : plus aucun pipeline refusé depuis `12e7c2b` (étage géométrique, coût mesuré nul). À rouvrir seulement si AGS mesure mieux. |
| T3 rapports RGA / RGP / RMV | À FAIRE | Après T1. |
| T4 quarantaine des 4 fichiers | FAIT | Rien n'en est repris ([SDK_INVENTORY.md](SDK_INVENTORY.md)). |

## Lot W — veille des dépôts

| Lot | État | Ce qui est fait, ce qui manque |
|---|---|---|
| W1 décision et installation | FAIT | DECISIONS 14. `veille.ps1`, tâche « PROJECT ONE veille » toutes les heures. Passes réelles : 1 h 15 (à la main, 44 requêtes) et 1 h 17 (par la tâche, code 0) ; mise en pause prouvée à 0 h 01 pendant un lancement. Manque : la preuve de deux passes à une heure d'écart (la prochaine est à 2 h 17). |
| W2 ce registre | FAIT | |

## Registre du contrat (G2)

But : 330 ms → 50 ms par image, soit −280 ms. Aucune promesse : ce tableau se remplit par mesure.

| Poste | Aujourd'hui (banc propre) | Cible | Gain visé | Gain mesuré |
|---|---|---|---|---|
| Mise bout à bout processeur / carte / recopie | 91–92 % du temps d'image | recouvrement | −130 ms | −54 ms avec `relais` (pas par défaut) |
| Dessins indirects des programmes mesh | 136 ms de carte | ~20 ms | −116 ms | 0 |
| Reste de la carte | ~64 ms | ~30 ms | −34 ms | 0 |
| Processeur (préparation + recopie) | 93–101 + 36–38 ms | ~45 ms | −85 ms | 0 |
| **Total** | **327–337 ms** | **50 ms** | **−280 ms** | **−54 ms disponibles en option, 0 par défaut** |

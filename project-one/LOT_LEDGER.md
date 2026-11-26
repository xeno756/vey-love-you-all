# Registre des lots

Un lot est FAIT, EN COURS, À FAIRE ou REFUSÉ (avec la raison). Mis à jour à chaque itération.
Les preuves sont dans [FINDINGS.md](FINDINGS.md), ce qui bloque dans [BLOCKERS.md](BLOCKERS.md),
les choix dans [DECISIONS.md](DECISIONS.md).

Dernière mise à jour : 26/11/2026 à 13 h : le monde s'affiche (lancement de 11 h 12, 5,6 images par seconde) ; visages noirs, ombres du soleil et attentes des textures traités, non vérifiés en jeu ([docs/technical/12-audit-moteur-26-11.md](docs/technical/12-audit-moteur-26-11.md)). Avant, à 11 h 15 : le monde était noir depuis 8 h 25 (collecte du brouillard laissée de côté) ; le brouillard est rendu de nouveau, les 6 images par seconde mesurées sans lui ne comptent pas. Préréglage SafeMode écrit, jamais lancé. Je ne lance plus le jeu.

## Mission 10 à 15 images par seconde

Objectifs : niveau 1, moyenne ≥ 10 images par seconde et 95 % des images sous 100 ms ;
niveau 2, ≥ 15 et 95 % sous 66,7 ms ; sans plantage ni gel, sur plusieurs lancements.
Depuis le 26/11 à 8 h 55 je lance de nouveau le banc moi-même (DECISIONS 29) ; avant, les
lancements étaient ceux de l'utilisateur. La version à lui donner est une seule :
`series-x performance`.

| Cycle | État | Ce qui est fait, ce qui manque |
|---|---|---|
| 1 cartes d'ombres écartées, profil `performance` | FAIT, MESURÉ | `6eaf67c`, `aad682c`, `a409572`. Deux lancements en banc propre : 4,68 et 4,45 images par seconde (201 et 217 ms) contre environ 3 (327–337 ms). Objectifs non atteints. Manque : une capture de contrôle (opérations par image). |
| 2 attentes de la carte | FAIT, MESURÉ (un lancement) | `19f3e56`. `performance precis relais:2` : 4,97 images par seconde, médiane 187 ms ; attentes 13 → 4 par image, mais 119 ms encore ; 0 alerte. Gain de 7 à 14 % : l'attente s'est déplacée vers la relecture. `fenetres` reste non mesuré. |
| 3 relecture de ce que les shaders écrivent | REFUSÉ | `vitesse:2` et `vitesse:3` la suppriment (8,8 images par seconde) mais perdent la carte : 6 lancements sur 7 (BLOCKERS 31). Retirés de `performance`. |
| 2 bis brouillard volumétrique | REFUSÉ | Sans sa collecte, le monde est noir (lancements de 8 h 25 à 10 h 38). Retiré de `performance` (`2273cfe`). Pour un brouillard moins cher : `volumetricsQuality` du jeu, par le préréglage. |
| 7 monde visible | FAIT, VÉRIFIÉ | Brouillard rendu (`2273cfe`). Lancement de l'utilisateur, 26/11 11 h 12 : le monde 3D s'affiche (deux captures), 5,64 images par seconde, médiane 175 ms. |
| 9 visages noirs (données streamées) | FAIT, NON VÉRIFIÉ | `84fe251`, deux correctifs de l'amont fxceless. Manque : une capture d'un PNJ de face. |
| 10 soleil à travers les murs | FAIT, NON MESURÉ | `b3dd536` : cascade proche dans `performance`, profils A/B/C. |
| 11 attentes de la carte (tampons d'envoi) | FAIT, MESURÉ (un lancement) | `3018756`, dans `performance` depuis `bb988d4` : 6,43 images par seconde contre 5,25, médiane 153 contre 188 ms, 300 s sans perte. |
| 13 murs manquants | FAIT, VÉRIFIÉ (une capture, 13 h 47) | `a5b2dd3`. Prix : 3,8 contre 6,7 images par seconde au même moment du monde (BLOCKERS 39). |
| 15 plantage title+0x4dbd74d | FAIT, NON VÉRIFIÉ | `0b3af3b` (écritures du noyau gardes ouvertes). |
| 16 personnages plus nets | FAIT, NON VÉRIFIÉ | `0b3af3b`, mot `personnages`. |
| 17 limiteurs d'images, lancer de rayons | ÉTUDIÉ | Doc 13 : aucun plafond ne joue dans le monde ; rayons déjà éteints. |
| 14 coût du miroir des descripteurs | EN COURS | `762c7b8` compte les sortes d'enregistrements refaits ; à traiter après le prochain lancement. |
| 12 fusion de l'amont fxceless | À FAIRE | 98 commits, 35 blocs en conflit sur la branche stable (BLOCKERS 37). |
| 8 préréglage SafeMode abaissé | FAIT, JAMAIS LANCÉ | `2cf979c`, option `prereglage:bas|extreme|diagnostic|safemode`. 11 tests hors jeu. Positions des champs non vérifiées en jeu (BLOCKERS 35). |
| 2 ter personnages incomplets | PRÉPARÉ, NON VÉRIFIÉ | `performance` ne laisse plus `transparence` de côté (`19f3e56`). À confirmer à l'œil et sur capture. |
| 3 manque de RAM | À FAIRE | 18 700 à 19 200 défauts de page par image avec `performance` (53 000 avec `rapide`), RAM libre au plus bas 10 à 24 Mo. Lot R2 (fenêtres page par page). |
| 5 attentes et passes de la carte, sans `vitesse:3` | FAIT, MESURÉ | 256 listes en vol, fenêtres qui passent les écritures des tampons (`9e52aa5`), comparaison de la carte par groupe (`a191a66`). |
| 6 stabilité | FAIT, MESURÉ (sur un monde noir) | Le relais au niveau 2 avec les attentes précises perdait la carte (10 sur 15) : retirés (`6fd9440`). `performance` : 6,1 et 6,0 images par seconde, 300 s tenu deux fois, mais sans brouillard, donc monde noir : la vitesse est à remesurer. |
| 4 dessins mesh regroupés | À FAIRE | BLOCKERS 14 : environ 2 300 dessins par image restent ; la carte travaille 110 ms par image. Après les mesures du cycle 2. |

### Registre de la mission

But : 330 ms → 100 ms par image (niveau 1), puis 66,7 ms (niveau 2).

| Poste | Aujourd'hui | Levier | Gain estimé | Gain mesuré |
|---|---|---|---|---|
| Cartes d'ombres, relais, réglages bas (profil `performance`) | 327–337 ms avec `rapide` | cycle 1 | vers 220–250 ms | **201 et 217 ms mesurés** (−120 ms, 4,5 images par seconde) |
| Attentes de la carte | 12,8–14 par image, 135–154 ms | `precis`, `relais:2` | vers 120–150 ms par image | **187 ms** (−14 à −30 ms) : 4 attentes, 119 ms |
| Relecture des écritures des shaders | 36 Mo par image, 60–104 ms d'attente | `vitesse:2`, `vitesse:3` | vers 120–140 ms par image | non mesuré |
| Brouillard volumétrique | 18,7 ms de carte (25/11) | `sans:volumetrie-collecte` | −15 ms | non mesuré |
| Bornage un par un | une passe de calcul par dessin indirect | `fenetres` | −10 ms | non mesuré seul |
| Dessins mesh restants | environ 2 300 par image, carte au travail 110 ms | regroupement | −30 à −50 ms | 0 |
| Processeur et recopie | 56–57 + 33–35 ms | RAM, copies | à chiffrer | 0 |
| **Total** | **201–217 ms (4,5 images par seconde)** | | **100 ms possibles dans le meilleur cas du cycle 2** | **cycle 1 : −120 ms ; cycle 2 : 0 vérifié** |

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

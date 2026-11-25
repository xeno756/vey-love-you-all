# 10 — Le premier lancement réussi de la branche fusionnée, et les profils qui en découlent

Suite de [09-plantages-du-25-11.md](09-plantages-du-25-11.md). Lancement du 25/11 à 9 h 41 :
`meilleure.cmd series-x`, sans option, donc la définition du jeu (rendu 3D en 2560 × 1440).
Tout ce qui suit vient de son journal.

## 1. Ce que ce lancement établit

| Fait | Valeur |
|---|---|
| La correction de l'erreur de fusion (document 09, section 4) | **confirmée en jeu** : le monde s'affiche, la carte n'est plus perdue à sa première image |
| Du démarrage au monde | 68 s |
| Temps passé dans le monde | 7 min 35 s |
| Images par seconde dans le monde | 0,2 à 3,1 ; la moitié du temps sous 1,0 |
| Fin du lancement | **manque de mémoire** (`a host command list did not reset`, code `0x8007000E`), pas une fermeture voulue |
| Le jeu a relu les shaders préparés | oui : même dossier de cache que la préparation |

## 2. Où part une image

Moyenne des 79 relevés dans le monde (un toutes les 5 secondes) :

| Poste | Temps par image | Part |
|---|---|---|
| Attente de la carte | 767 ms | 65 % |
| Préparation par le processeur (copies de mémoire du jeu, textures) | 182 ms | 15 % |
| Recopie vers la mémoire du jeu après l'exécution | 89 ms | 8 % |
| Le reste (travail du jeu, attentes, mémoire sur disque) | 147 ms | 12 % |

La carte est le premier frein. Les deux postes du processeur font à eux seuls 271 ms : même
avec une carte instantanée, ce lancement n'aurait pas dépassé 3 à 4 images par seconde.

## 3. La mémoire, cause de la fin du lancement et des textures absentes

| Mesure | Début du monde | Fin |
|---|---|---|
| Mémoire de la carte utilisée, pour un budget de 4,6 Go | 5,5 Go | 6,5 Go |
| Mémoire du PC partagée avec la carte | 4,6 Go | 6,7 Go |
| Mémoire demandée par le processus, pour 15,8 Go de RAM | 17,4 Go | 21,2 Go |
| Textures en attente de place sur la carte | 28 900 | 39 100 |
| Textures chargées | 2 297 (165 Mo) | 6 596 (439 Mo) |

La carte dépasse son budget dès la première image du monde. Tant qu'elle le dépasse, le
runtime ne lui envoie plus de textures (`the card has no room for more; they wait`) : en
7 min 35 s, une texture sur sept seulement a été chargée. C'est ce qui donne des modèles sans
leurs textures et un monde qui semble ne jamais finir de charger. La demande totale (carte
plus mémoire partagée) atteint 13,1 Go pour une carte de 6 Go : le lancement s'arrête quand
Windows ne peut plus en donner.

Les tailles des images que le jeu calcule suivent bien la définition demandée : au lancement
`1080p` de 9 h 00, les tampons du monde étaient en 1920 × 1080 et ses fractions, plus du
2560 × 1440.

## 4. Ce qui manque à l'image

| Mesure | Valeur |
|---|---|
| Dessins sans pipeline, par image | 160 à 3 800 |
| Exécutions indirectes laissées de côté, par image | 100 à 700 |
| Pipelines de programmes mesh faits en jeu | 806 |
| Pipelines refusés par la carte | 6 |

Les 6 pipelines refusés sont connus : 4 ont un pixel shader qui lit les coordonnées
barycentriques, que la RX 5600 XT n'a pas (prépasse alpha et transmission de la peau, masque
d'essuie-glace, une variante texturée) ; les 2 autres sont les cheveux en mèches et un filtre
de halo. Ce journal ne dit pas quels pipelines manquent aux autres dessins : la version
diagnostic (`profil`) le dit.

## 5. Les profils ajoutés au lanceur

Ils règlent des options qui existaient déjà, d'après ces mesures. **Aucun n'a encore tourné
en jeu.**

| Commande | Ce qu'elle règle | Ce qu'elle vise | Ce qu'on perd |
|---|---|---|---|
| `meilleure.cmd series-x rapide` | image et rendu 3D en 1280 × 720 ; textures sans leurs 2 premiers niveaux ; 4 groupes d'effets laissés de côté (particules, vent, flou et halos, reflets) | 4 fois moins de pixels que 2560 × 1440 pour les calculs plein écran et leurs tampons ; textures 16 fois plus légères, donc toutes chargeables | netteté, particules, reflets |
| `meilleure.cmd series-x maximum` | 960 × 540 ; textures sans 3 niveaux ; 8 groupes sur 9 (tous sauf la volumétrie, sans laquelle l'image peut être noire) | 7 fois moins de pixels ; 64 fois moins de texels | en plus : ombres lointaines, décalcomanies, transparences et lumière rebondie, qui peuvent être faux ou figés |
| ajouter `vitesse` | deux réglages de ScarlettPort du côté du processeur : contrôles de contenu plus légers, copies évitées quand l'écriture est déjà connue | les 271 ms du processeur | ScarlettPort ne les a pas mesurés ; à retirer en premier si l'image se dégrade |
| ajouter `vitesse:2` | en plus, ce que les shaders écrivent n'est recopié vers la mémoire du jeu que là où le jeu le relit | les 89 ms de recopie | le plus risqué des trois : le jeu peut lire des données périmées |

Aucun de ces profils ne retire un dessin : les modèles restent tous demandés à la carte.

Ordre d'essai conseillé, un changement à la fois : `rapide`, puis `rapide vitesse`, puis
`maximum`.

## 6. Pipelines de dessin compilés d'avance

Fait le 25/11 à 10 h 01 avec la version corrigée (`meilleure.cmd series-x
preparer-graphiques`, le jeu n'est pas lancé) :

| Mesure | Valeur |
|---|---|
| Enregistrements du cache de pipelines du jeu | 25 471 |
| Pipelines de dessin faits et gardés dans la bibliothèque | 24 801 |
| Refusés par la carte | 403 (les barycentriques, section 4) |
| Doublons, et shaders absents de l'archive | 246 et 21 |
| Durée | 58 s, après 28,5 s pour les shaders et les pipelines de calcul déjà préparés |

Un pipeline que le jeu demande avec les mêmes shaders et le même état est alors relu au lieu
d'être compilé (2 à 3 secondes chacun au lancement de 8 h 59). Combien des dessins du monde
en profitent n'est pas mesuré : le prochain journal le dira (lignes `pipeline library:
loaded` contre `made`).

## 7. Ce qui n'est pas fait, et pourquoi

- **Réduire les polygones.** Le choix du niveau de détail des modèles appartient au jeu. Son
  exécutable ne garde aucun nom de réglage (recherche de « lodscale », « lodbias »,
  « shadowquality », « graphicssettings » et voisins : rien), donc pas de réglage à modifier sans l'étudier fonction par fonction. Le jeu a en
  revanche son propre préréglage bas : celui de la Series S, que 70 endroits de son code
  consultent. L'activer tout en gardant la mémoire de la Series X est une piste, pas encore
  ouverte : elle touche au mode Series S, mis de côté le 25/11.
- **Les 4 pipelines refusés faute de barycentriques.** Il faut que le shader de sommets
  généré pour les programmes mesh donne lui-même ces coordonnées au pixel shader.
- **Les copies du processeur elles-mêmes** (100 à 200 Mo copiés et 380 Mo comparés par image) :
  `vitesse` n'active que des réglages existants.

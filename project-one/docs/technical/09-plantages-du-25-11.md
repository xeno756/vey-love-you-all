# 09 — Les plantages du 25/11 au matin et leurs causes

Suite de [08-resolution-scarlettport-intro.md](08-resolution-scarlettport-intro.md). Vos lancements
de 8 h 20 à 8 h 27 ont donné trois défauts : `1080p` et `720p` se ferment au démarrage, la
branche fusionnée se ferme à l'entrée du monde, l'introduction saccade sur la branche stable
et pas sur la branche fusionnée. Les trois causes sont établies à partir de vos journaux, et
deux sont reproduites ou vérifiées hors jeu.

**Les corrections ci-dessous n'ont pas encore tourné avec PROJECT GAME.**

## 1. `1080p` et `720p` : une vérification interne du jeu

| Fait | Source |
|---|---|
| Le hook de taille fonctionne : le jeu avait choisi 3840 × 2160, il a reçu 1920 × 1080 | ligne `resolution: the title chose its own size…` |
| 3,5 s après le démarrage, le jeu s'arrête sur un point d'arrêt à lui (`+0x247af2d`) | ligne `first-chance exception 0x80000003` |
| Ce point d'arrêt suit une comparaison : la mémoire que le jeu a réservée pour ses images de sortie (64 Mio en Series X) doit être utilisée à 2 Mio près | désassemblage de la fonction `0x247ad10` |
| En 1920 × 1080 il reste 49 Mio, en 1280 × 720 il en reste 56 | valeur du registre dans les deux journaux |

Le jeu vérifie donc que ses images de sortie ont la taille qu'il attend, et s'arrête sinon. Ce
n'est pas une panne du runtime.

Correction (`fc73e53`) :

- cette vérification précise ne peut plus arrêter le jeu ;
- tant qu'une taille lui est imposée, tout autre point d'arrêt de l'exécutable du jeu est
  franchi et écrit dans le journal avec sa position (`resolution: a check of the title's own
  stopped it…`). S'il en existe d'autres, un seul lancement les montrera tous.
  `VICTORY_TITLE_CHECKS=1` les rend à nouveau bloquants.

Ce qui reste incertain : qu'aucune autre vérification ne mène à un vrai plantage plus loin.

## 2. Branche fusionnée : un programme que le pilote AMD ne finit pas de compiler

| Fait | Source |
|---|---|
| La carte est perdue 71 s après le démarrage, à l'entrée du monde, après 119 ms d'attente : ce n'est pas un dépassement de temps | ligne `host GPU device removed` |
| À cet instant un fil compilait depuis 17,6 s le pipeline d'un programme de calcul de l'éclairage indirect (activation des rayons des sondes d'écran, 66 501 instructions) | même ligne, `make of … running for 17622.4 ms` |
| ScarlettPort note que, traduit sans les passes de nettoyage, ce programme fait prendre 0,8 Go de plus toutes les 10 s au compilateur d'AMD, sans fin | commentaire de leur code |
| **Reproduit hors jeu** : 1,1 Go après 5 s, 2,1 après 10 s, 3,2 après 21 s, 4,4 après 37 s, toujours en cours | essai du 25/11 avec la branche fusionnée, ce seul programme |
| Votre PC a environ 1 Go de RAM libre en jeu | lignes `frame account` de la branche stable |

La branche stable n'a pas ce défaut parce que le fork de fxceless nettoie les shaders traduits
par défaut. ScarlettPort a désactivé ce nettoyage : il fait disparaître les murs, le toit et le
sol de la première pièce sur plusieurs cartes. La branche fusionnée garde le réglage de
ScarlettPort (image juste) et, sur une carte AMD, **ne compile plus ce programme** : ses
calculs sont laissés de côté, comme le fait déjà le groupe `eclairage-indirect`.
`VICTORY_ACTIVATE_RAYS=1` le remet, `VICTORY_SHADER_OPT=on` remet le nettoyage.

C'est aussi ce programme qui avait fait durer la préparation de cette branche 18 min 47 s.

**Correction du même jour.** Ce programme laissé de côté, vos lancements de 8 h 59 et 9 h 00
ont perdu la carte au même instant. La mémoire qu'il prend est réelle et reproduite, mais ce
n'est pas lui qui faisait perdre la carte : la cause est en section 4.

## 3. Introduction : ma première explication était fausse

J'avais attribué les saccades à la présentation des images sans calage sur l'écran. Vos
journaux le démentent :

| Mesure sur la branche stable, pendant l'introduction | Valeur |
|---|---|
| Fréquence de votre écran | **280 Hz** : un rafraîchissement toutes les 3,6 ms, le calage n'y change presque rien |
| Écart entre deux présentations | 33,1 à 33,6 ms en moyenne, la plus longue à 34,3 ms |
| Présentations en retard | 0 |
| Présentations en avance (moins de 20 ms après la précédente) | 0 à 3 par tranche de 5 s |

Les images sortent donc régulièrement, et pourtant l'animation saccade. La différence avec la
branche fusionnée est ailleurs : le jeu demande au système **à quels instants ses images ont
été affichées** (`GetFrameStatisticsX`). La branche stable ne répond rien ; la pile de
ScarlettPort tient ces instants et les lui donne. Sans eux, le jeu ne peut pas régler le pas de
ses animations. C'est une lecture du code, cohérente avec ce que vous avez vu sur les deux
branches ; je n'ai pas isolé ce seul changement en jeu.

Ce que le compteur vidéo a montré par ailleurs :

- avant le logo, aucun film n'avance : la séquence d'ouverture est dessinée par le jeu
  (illustrations, zooms, flous), ce n'est pas une vidéo ;
- le premier film lu l'est à 13 h 25, dans le monde : 30 images par seconde, 0 image sautée,
  4,7 ms de décodage par image ;
- à 13 h 25 min 05 s un second lecteur est ouvert deux fois, et refermé chaque fois 50
  microsecondes plus tard : le jeu lit l'en-tête d'un film, il ne le joue pas. **Deux films ne
  sont jamais lus en même temps.**

Le calage sur l'écran reste en place : il ne coûte rien et sert sur un écran à 60 Hz.

## 4. Lancements de 8 h 59 et 9 h 00 : la carte perdue à la première image du monde

| Fait | Source |
|---|---|
| Les deux lancements de la branche fusionnée (normal et `1080p`) passent l'introduction et le chargement, puis perdent la carte à la première image du monde, 65 s après le démarrage | ligne `host GPU device removed` ; la dernière ligne `fps` est celle de l'écran de chargement |
| La carte a gardé la soumission 54 ms (7 ms en `1080p`) : pas un dépassement de temps, une commande qu'elle ne peut pas exécuter | même ligne, code `0x887A0006` |
| `1080p` ne s'arrête plus sur la vérification du jeu | journal de 9 h 00 |
| La branche stable, elle, entre dans le monde et y tourne (lancement de 8 h 20, modèles 3D visibles) | son journal |

La cause est une erreur de ma fusion, pas un défaut de ScarlettPort ni de la carte.

Le traducteur de ScarlettPort donne à presque tous les shaders une table que le runtime doit
leur brancher (les « mots des descripteurs » : tailles, niveaux et adresses des vues, que le
jeu lit comme des nombres). Elle porte le numéro d'emplacement 252. Notre code des programmes
mesh exécutés en calcul utilisait déjà ce numéro 252 pour son tampon d'enregistrements, et dit
au runtime : « l'emplacement 252 n'est pas à brancher ici, le dessin mesh s'en charge ». Dans
la branche stable ce numéro était libre. Dans la fusion, la table de ScarlettPort n'était donc
**jamais branchée, pour aucun shader** : chaque shader qui la lisait lisait une adresse
quelconque.

| Mesure, hors jeu | Valeur |
|---|---|
| Shaders du jeu qui demandent cette table | 23 935 sur 25 563 (94 %) |
| Parmi les 1 171 shaders que vos deux lancements ont utilisés | 468 de calcul sur 468, 178 mesh sur 178, 357 pixel sur 399, 93 de sommets sur 126 |
| Dans l'arbre de ScarlettPort | la table est branchée à chaque dessin et chaque calcul |
| Dans la fusion | le test « 252 = tampon mesh, on passe » était placé avant : le branchement n'était jamais atteint |

Ces nombres viennent de la lecture du cache de shaders de la branche fusionnée (chaque
traduction y garde la liste de ses emplacements) et de la comparaison des deux fichiers
`draw.rs`.

Correction : le tampon mesh prend l'emplacement 251, et la compilation échoue désormais si ce
numéro rejoint un jour ceux du traducteur. Un second numéro partagé (le mot « premier groupe »
d'un calcul découpé, `0x104`) désignait la même chose des deux côtés : il est maintenant un
seul nom, sans changement d'effet.

Un second défaut est apparu en cherchant : sur la branche fusionnée, **la préparation ne
servait pas au jeu**. La clé du cache contient une limite que le runtime demande à la carte ;
en jeu elle était calculée avant cette demande, en préparation après. Le jeu rangeait donc ses
traductions dans un autre dossier et retraduisait tout : les 1 171 shaders de vos lancements
ont été refaits, identiques octet pour octet à ceux déjà préparés. La clé est maintenant
calculée après la demande dans les deux cas, et le journal écrit le nom du dossier utilisé
(`shader cache: this build's translations`).

Ce qui n'est pas prouvé : que cette table non branchée soit bien la commande qui a fait
perdre la carte. C'est une lecture du code appuyée par des mesures, cohérente avec le moment
du plantage (la première image qui utilise ces shaders en masse) et avec son code d'erreur ;
seul un lancement le confirmera. Si la carte est encore perdue, `meilleure.cmd series-x profil`
(version diagnostic) écrit dans le journal la dernière commande exécutée.

## 5. Quelle version lancer

Deux versions existent côte à côte. Ce qui est vu en jeu, à 9 h 30 :

| Version | Commande | Vu en jeu |
|---|---|---|
| Branche stable | `lancer.cmd series-x` | entre dans le monde et y tourne, modèles 3D visibles, 0,3 à 2,5 images par seconde pendant les 25 secondes de monde du lancement de 8 h 20 ; introduction saccadée à ce lancement, corrigée depuis sans nouvel essai |
| Branche fusionnée avec ScarlettPort | `meilleure.cmd series-x` | introduction fluide ; carte perdue à la première image du monde (8 h 26, 8 h 59, 9 h 00) ; l'erreur de fusion de la section 4 est corrigée, pas encore réessayée |

`lancer.cmd` et `meilleure.cmd` sont dans `project-one` et prennent les mêmes options (`1080p`,
`720p`, `minimal`, `profil`…).

Mesuré hors jeu sur la branche fusionnée corrigée (`30cba67`) : 756 tests réussis, 0 échec ;
préparation des shaders à froid en 5 min 18 s : 25 563 shaders traduits sur 25 563, 10 498 pipelines (calcul et moitié calcul des programmes mesh) dans la bibliothèque, 0 refusé par la carte.

Ce que la correction change pour les fichiers déjà préparés : les pipelines graphiques du jeu
compilés d'avance le matin (`preparer-graphiques`, 24 801) sont à refaire pour les programmes
mesh, puisque le shader de sommets qui lit leur tampon a changé (déduit de la façon dont la
bibliothèque nomme ses pipelines, pas mesuré). Ces pipelines se compilent en jeu à leur
premier usage, comme sur la branche stable : à la première entrée dans le monde, des objets
manquent puis apparaissent pendant quelques minutes ; le lancement suivant les relit.
`preparer-graphiques` les refait d'avance.

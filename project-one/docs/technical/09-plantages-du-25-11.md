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

## 4. La meilleure version

C'est la branche fusionnée (`project-one/scarlettport`, dossier `..\victory-scarlettport`) avec
ces corrections : elle a l'introduction fluide, traduit tous les shaders du jeu, compile
d'avance les pipelines graphiques et contient tout ce que PROJECT ONE avait ajouté.

| Commande (lanceur de la branche fusionnée) | Effet |
|---|---|
| `lancer.cmd series-x` | comme hier, sans le programme que le pilote ne compile pas |
| `lancer.cmd series-x 1080p` | image et rendu 3D en 1920 × 1080 |
| `lancer.cmd series-x 720p minimal` | 1280 × 720, textures et effets réduits |

La branche stable reste telle quelle, en secours.

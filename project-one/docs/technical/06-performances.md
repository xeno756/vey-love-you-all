# 06 — Pourquoi environ 1 image par seconde, et ce qui a été changé

Date : 24/11/2026. Machine : Ryzen 7 5700 (8 cœurs, 16 fils), 15,8 Go de RAM, RX 5600 XT 6 Go
(RDNA1, sans mesh shaders).

Tout ce qui est marqué **mesuré** vient d'un journal ou d'un banc d'essai lancé sur cette
machine. Ce qui est marqué **hypothèse** n'a pas été vérifié. Aucun essai n'a lancé PROJECT GAME
depuis cette session : les lancements sont faits par l'utilisateur.

## 1. Les journaux disponibles

| Lancement | Version | Durée | Disponible ? |
|---|---|---|---|
| Fork d'origine et versions intermédiaires (avant le 24/11 à 22 h) | — | — | **non** : le journal était réécrit à chaque lancement (`%TEMP%\victory-err.log`) |
| Series S, version normale, 22 h 00 | PROJECT ONE `6fac177` | 35 s | non (réécrit), lu en direct avant |
| Series S, version normale, 22 h 04 | PROJECT ONE `6fac177` | 82 s | **oui** |
| Series X, version diagnostic avec capture, 22 h 11 | PROJECT ONE `915977f` | 7 min | **oui**, avec 1 150 images et 4 listes d'opérations |
| XCL (sept journaux du 24/11) | autre runtime | — | oui, non comparables ligne à ligne |

Les deux journaux gardés sont dans `..\victory-logs`. Depuis cette session, le lanceur garde un
journal par lancement dans ce dossier.

Conséquence : la comparaison « avant / après » entre le fork et PROJECT ONE ne peut pas être
faite sur des journaux. Elle repose sur ce que la session a observé en direct : avant le
remplacement des mesh shaders, 45 à 65 % des dessins étaient refusés.

## 2. Ce que montre le lancement Series X de 7 minutes

### Le remplacement des mesh shaders fonctionne

- 444 programmes mesh traduits en calcul, 0 échec de traduction ;
- 693 pipelines mesh créés, 3 refusés par la carte ;
- 150 à 250 dessins mesh directs par image sont exécutés.

Mais 2 000 à 2 260 dessins mesh **indirects** par image étaient encore sautés. C'est la
géométrie solide du monde : les images capturées montrent les lumières de la ville et le texte
du générique, sans les bâtiments.

### Où part le temps d'une image (mesuré)

Le jeu envoie son travail à la carte sur un seul fil, en série : préparer, envoyer, attendre la
carte, recopier ce que les shaders ont écrit. La somme de ces trois temps fait 94 à 98 % du
temps de l'image sur les images vérifiées.

| Phase du lancement | Temps par image | Préparation | Attente de la carte | Recopie | Cœurs occupés |
|---|---|---|---|---|---|
| Écran de lancement et chargement (80 s) | 32 à 68 ms, pointes à 250 à 340 ms | 1 à 9 ms | 3 ms | 0 | 4 à 18 |
| Monde, pipelines en cours de compilation (4 min 30) | 0,7 à 3,9 s, pointes à 16 à 20 s | 0,2 à 3,3 s | 0,2 à 3,5 s | 0,05 à 0,7 s | 7 à 19 |
| Monde, pipelines tous compilés (50 s) | 0,7 à 3,4 s | 0,16 à 0,46 s | 0,3 à 3,0 s | 0,05 à 0,18 s | **0,5 à 4** (jusqu'à 7 quand une compilation reprend) |

Lecture :

- **Quand les pipelines sont compilés, le processeur est presque inactif** (0,5 à 4 cœurs sur
  16) et l'image attend deux gros envois à la carte, de 0,4 à 2,6 s à eux deux. La limite est
  alors la carte, pas le processeur.
- **Pendant 4 min 30, les fils d'enregistrement attendent 500 ms par image** que des pipelines
  finissent de compiler (c'est le budget d'attente par image), puis l'image saute quand même
  650 à 4 180 dessins. La compilation avance à 3,5 pipelines par seconde avec 8 fils. Cette
  attente se déroule en grande partie pendant que le fil d'envoi attend la carte : quand la
  carte est chargée, elle allonge peu l'image (735 ms avec l'attente, 680 ms sans, sur deux
  images comparables). Quand la carte est libre, comme à l'écran de lancement, elle peut faire
  toute l'image : 520 ms par image au début du lancement Series S, avec un dessin sauté faute de
  pipeline (déduction : ce journal ne détaille pas l'attente).
- **Les images de 16 à 20 s** coïncident avec 1,3 à 5,7 millions de défauts de page.

### La mémoire (mesuré)

| Mesure dans le monde, Series X | Valeur |
|---|---|
| Mémoire demandée par le processus | 19 à 21,6 Go |
| RAM de la machine | 15,8 Go |
| Part du processus réellement en RAM | 4,2 à 4,8 Go |
| RAM libre | 0,7 à 2,2 Go |
| Mémoire vidéo utilisée | 5,3 à 6,6 Go pour un budget de 4,85 Go (carte de 6 Go) |
| RAM prise par la carte comme mémoire partagée | 5,0 à 5,9 Go |
| Défauts de page par image | 150 000 à 650 000 |

Le jeu en mode Series X dispose de 13,5 Go pour lui seul. Avec les copies destinées à la carte,
le total dépasse la RAM de 4 à 6 Go : une grande partie du jeu vit dans le fichier d'échange. La
mémoire vidéo dépasse aussi son budget de 0,5 à 1,8 Go, donc Windows en déplace une partie en
RAM, ce qui ralentit la carte.

### Ce que le jeu demande à la carte par image (mesuré sur les images capturées)

| Travail | Par image |
|---|---|
| Listes de commandes | 84 à 117 |
| Calculs directs | 700 à 1 440, soit 1,8 million de groupes |
| dont passes plein écran en 2560 × 1440 | environ 30 |
| Calculs indirects | 740 à 1 360 |
| Dessins mesh directs | 150 à 250 |
| Dessins mesh indirects | 2 000 à 2 260 (sautés jusqu'ici) |
| Autres dessins | 190 à 240 |

Une passe de calcul de 160 × 90 groupes prend 8,7 ms sur cette carte (mesuré dans le journal).
Les passes plein écran en font quatre fois plus (320 × 180 groupes) : à coût égal par groupe,
trente passes prennent de l'ordre d'une seconde (estimation).

### Réponses aux observations

| Observation | Cause |
|---|---|
| Environ 1 image par seconde dans le monde | la carte : 0,4 à 2,6 s par image pour le rendu en 1440p du mode Series X, aggravé par la mémoire vidéo dépassée. Pendant les 4 à 5 premières minutes s'ajoutent la compilation des pipelines (7 à 19 cœurs occupés) et les images de 16 à 20 s dues à la mémoire qui déborde |
| Les sous-titres ralentissent | ils avancent avec les images du jeu : à 1 image par seconde, tout ce que le jeu met à jour par image ralentit. Le fil principal n'est pas bloqué en dehors de l'attente de la carte |
| L'écran de lancement est lent, le chargement moins | **expliqué en partie.** Mesuré : dans les premières secondes, les images durent 340 à 520 ms alors que la carte est libre (0 à 4 % d'occupation) ; ensuite le jeu tient 25 à 30 images par seconde jusqu'au monde. Deux causes vues : une image qui dessine avec un pipeline pas encore prêt l'attend jusqu'à 500 ms (lancement Series S : 520 ms par image, un dessin sauté, déduit du budget d'attente) ; et le jeu lui-même n'envoie rien pendant son démarrage (lancement Series X : 344 ms par image, 4 cœurs occupés). En plus, 1 200 à 1 600 pipelines connus des lancements précédents sont compilés d'avance par 8 fils pendant cette phase (4 à 18 cœurs occupés) |
| Le monde est incomplet | 2 000 à 2 260 dessins mesh indirects sautés par image, plus les dessins dont le pipeline n'est pas encore compilé |

Part de chaque facteur, dans l'état stable du monde en Series X : la carte représente 60 à 90 %
du temps de l'image, la préparation et la recopie par le processeur 10 à 40 %. La compilation
des shaders ne pèse que pendant les premières minutes.

## 3. Le mode Series S : une cause probable du blocage, corrigée

En mode Series S, le jeu choisit lui-même ses réglages pour une console de 4 téraflops et 8 Go :
moins de mémoire et une définition interne plus basse. C'est le mode le plus proche de cette
machine. Il restait sur un écran noir.

Mesuré dans le journal de 82 s :

- le jeu présente 2 380 images de son premier écran (13 dessins par image, 30 images par
  seconde) et n'ouvre aucun fichier après `core_licenses_data.rpf` ;
- en Series X, il ouvre le fichier suivant 10 s après celui-là ;
- la seule question que le jeu pose en Series S et pas en Series X est « combien de mémoire
  est-ce que j'utilise ? » ;
- la réponse était : 6 082 Mo utilisés sur 7 776 Mo, trois secondes après le démarrage.

Cause probable : le runtime répondait avec toute la mémoire du processus, la sienne et les
copies pour la carte comprises, et ce chiffre grossit à mesure que le runtime travaille. Le code
du fork le signalait lui-même comme un raccourci à corriger. Le jeu se croit alors presque plein,
puis au-dessus de sa limite.

Ce qui n'est pas établi : la part du jeu dans les 6 082 Mo (le journal ne la donnait pas), ni
que ce chiffre soit la seule raison du blocage.

Correction (`a7e55ef`) : la réponse compte maintenant la mémoire du jeu seul, c'est-à-dire ce
qu'il a engagé dans ses propres réservations et les pages physiques qu'il détient.
`VICTORY_TITLE_USED=process` remet l'ancienne réponse.

**Non vérifié** : que le jeu dépasse bien son premier écran avec cette réponse. C'est le premier
essai à faire. Le journal donne maintenant, pour les huit premières questions du jeu puis une
sur 256, la mémoire du jeu, ses pages physiques et celle du processus : si le blocage persiste,
ces lignes diront pourquoi.

## 4. Les changements de cette session

| Commit | Changement | Vérifié comment |
|---|---|---|
| `a7e55ef`, `a89d712` | Mémoire dite au jeu : la sienne, pas celle du processus ; journal de ses questions | 2 tests ; pas vérifié en jeu |
| `1321976` | Dessins mesh indirects exécutés (un enregistrement par appel) ; un seul tampon d'enregistrements de 32 Mio pour toutes les listes ; banc d'essai du schéma de dessin mesh | test sur la carte de la passe de conversion ; pas vérifié en jeu |
| `8a1b109` | Les pipelines demandés par l'image en cours sont compilés avant les anciens | 1 test |
| `6946e41` | Cache de shaders rangé par version, versions périmées retirées | 1 test |
| `2580f18` | Commande `prepare` : traduit les shaders et compile les pipelines de calcul avant le lancement | lancée sur cette machine, voir section 5 |
| `7dc3266` | Lanceur : modes `profil` et `preparer`, un journal par lancement | essais à blanc |

Fichiers touchés : `crates/victory-hle/src/modules/xmem.rs` ; dans `crates/victory-gpu/src` :
`draw.rs`, `indirect.rs`, `host.rs`, `bench.rs` (nouveau), `shader_cache.rs`, `pipeline.rs`,
`pipeline/host.rs`, `pipeline/make.rs`, `pipeline/library.rs` ; `apps/victory/src/prepare.rs`
(nouveau) et `main.rs` ; `project-one/lancer.cmd`.

Un défaut introduit puis corrigé pendant la session : la première version de `prepare`
écrivait une liste vide à la place de la liste des pipelines à démarrer tôt, construite par les
lancements précédents (2 155 entrées). La liste se reconstruit au prochain lancement ; d'ici
là, les pipelines de dessin sont compilés à leur première utilisation et non pendant le
chargement. Le défaut est corrigé dans `2580f18`.

Détail des dessins mesh indirects : la carte lit le nombre de groupes demandé par le jeu. Une
petite passe de calcul le borne à la place disponible dans le tampon (2 200 groupes pour un
programme courant), puis écrit les arguments d'un calcul et d'un dessin, exécutés l'un après
l'autre. Les appels à plusieurs enregistrements ou avec compteur restent sautés, avec un message
dans le journal.

Pourquoi un seul tampon : avec un tampon par liste, chacune des 630 listes du jeu qui avait
enregistré un dessin mesh gardait jusqu'à 8 Mio de mémoire vidéo.

## 5. Bancs d'essai (mesurés sur cette machine)

### Schéma de dessin mesh

Question : faut-il regrouper les dessins mesh d'une série (un seul changement d'état) au lieu
de calcul, barrière, dessin, barrière pour chacun ?

| Charge | Un par un | Regroupés | Gain |
|---|---|---|---|
| 256 dessins de 4 groupes | 1,81 ms | 0,95 ms | 1,9 fois |
| 256 dessins de 18 groupes | 2,65 ms | 1,28 ms | 2,1 fois |
| 2 304 dessins de 2 groupes | 5,38 ms | 1,14 ms | 4,7 fois |

Le schéma un par un coûte 2 à 10 µs par dessin, soit 2 à 5 ms par image. Le regroupement
ferait gagner quelques millisecondes sur des images de 400 à 3 000 ms. **Décision : pas de
réécriture.** Commande : `cargo test -p victory-gpu --release bench -- --ignored --nocapture`.

### Traduction des shaders

| Mesure | Valeur |
|---|---|
| Shaders dans l'archive du jeu, lisibles avant tout lancement | 26 503 (2 236 vertex, 12 101 pixel, 9 282 mesh, 2 689 calcul) |
| Traduction de tous, sur 12 fils | 2 min 40 s |
| Programmes vertex, pixel, mesh et calcul traduits | 25 598 sur 25 694 (99,6 %) |

### Préparation avant le lancement (`victory prepare --compute-pipelines`)

Mesuré sur cette machine, cache de shaders vide, 14 fils :

| Mesure | Valeur |
|---|---|
| Recherche des shaders dans les fichiers du jeu | 25 985 conteneurs distincts en 4,6 s |
| Shaders des étapes qu'un lancement traduit | 25 563, dont 25 466 traduits et 97 non traduisibles |
| Pipelines de calcul dans la bibliothèque | 2 723, dont 1 084 y étaient déjà (faits par les lancements précédents), 0 refusé |
| Durée totale | 9 min 41 s |
| Taille des traductions d'une version | 1,3 Go |

Deux vérifications que la préparation sert bien au jeu :

- 995 des 996 shaders de calcul que le jeu a traduits pendant le lancement de 7 minutes sont
  parmi les shaders préparés (même empreinte) : le jeu passe bien les mêmes octets ;
- 1 084 pipelines ont été retrouvés dans la bibliothèque sous le nom que le jeu leur avait
  donné : un pipeline préparé porte donc le même nom qu'en jeu.

**Non vérifié** : le gain en jeu. Il se lira dans le journal : la ligne
`shaders from the cache and translated so far hits=… misses=…` et le nombre de pipelines
« being made ».

Ce que la préparation ne peut pas faire : les pipelines de dessin (vertex ou mesh, avec un
pixel shader) dépendent de l'état graphique que le jeu ne donne qu'en cours de partie. Ils se
compilent encore en jeu la première fois, puis la bibliothèque les garde.

### Cache de shaders

Les traductions sont rangées par version du runtime (dossier `builds` du cache de shaders, un sous-dossier par version). Le cache
garde la version en cours et celle utilisée juste avant (version normale et version
diagnostic), et retire les autres ainsi que les entrées de l'ancien rangement. Avant, chaque
recompilation ajoutait ses fichiers pour toujours, soit 1,3 Go par version préparée.

## 6. Ce qui limite encore

1. **La carte en mode Series X** : 0,4 à 2,6 s par image, processeur inactif. Aucun réglage du
   runtime ne ramène cela à 100 ms. Piste : le mode Series S, puis un profil de la carte par
   commande (`lancer.cmd series-s profil`) pour trouver les passes les plus lentes.
2. **La mémoire en mode Series X** : 20 Go demandés pour 15,8 Go. Piste : le mode Series S
   (8 Go pour le jeu).
3. **Le travail en série sur le fil d'envoi** (préparer, attendre, recopier). Le fork a un mode
   expérimental qui laisse la carte travailler pendant la préparation suivante
   (`VICTORY_LATE_WAIT=1`), désactivé par défaut et non essayé ici.

Objectif de 10 images par seconde : **non atteint, et non mesurable sans nouvel essai**. En mode
Series X sur cette machine il n'est pas atteignable : la carte seule prend 4 à 26 fois le temps
visé. En mode Series S, l'estimation est de 2 à 6 images par seconde dans le monde
(hypothèse : 4 fois moins de pixels, mémoire dans les limites), à mesurer.

## 7. Xbox Series S

### Faire tourner PROJECT ONE sur une console Series S

| Question | Réponse |
|---|---|
| Est-ce utile ? | Non. PROJECT ONE remplace le système de la console pour un PC. Sur la console, le jeu tourne déjà sans traduction. |
| Est-ce possible ? | Une console de série n'exécute que des paquets signés. Le mode développeur n'accepte que des applications UWP, dans un bac à sable, sans les interfaces dont PROJECT ONE dépend. |
| Modules propres à Windows | le chargeur (`victory-loader`), les services (`victory-hle`), le module graphique (`victory-gpu`, D3D12 du PC) : tout sauf le traducteur de shaders, qui n'aurait rien à traduire |
| Recommandation | **abandonner cette piste** |

### Se servir du profil Series S du jeu sur le PC

| Point | Series S | Series X | Cette machine |
|---|---|---|---|
| Carte | RDNA2, 4 téraflops | RDNA2, 12 téraflops | RDNA1, environ 7 téraflops, sans mesh shaders |
| Mémoire pour le jeu | 7,8 Go | 13,5 Go | 15,8 Go de RAM, 6 Go de mémoire vidéo |
| Définition interne | plus basse, avec mise à l'échelle FSR | 2560 × 1440 mesuré | — |

Le profil Series S est celui qui correspond à cette machine. **Recommandation : poursuivre**,
c'est la piste la plus prometteuse. Le blocage trouvé est corrigé (section 3), à vérifier.

Rien n'est transférable du matériel lui-même : les optimisations propres à la console sont dans
le jeu, pas dans le runtime.

## 8. Les autres projets

| Projet | Examiné ? | Ce qui en est retenu |
|---|---|---|
| VIctory, fork de fxceless (base de PROJECT ONE) | oui, code lu | toute l'architecture ; ses modes expérimentaux (`VICTORY_LATE_WAIT`, `VICTORY_INPLACE`) sont des pistes déjà écrites, non essayées ici |
| XCL | oui, code lu plus tôt dans la session (licence GPL-2.0 seule : aucune ligne reprise) | idées notées dans [ARCHITECTURE.md](../../ARCHITECTURE.md) : partage des pages de mémoire vidéo entre render targets, compilation limitée selon la mémoire libre |
| Scarlett Layer, Xbox2Windows, TuxBox, ScarlettX, ZTRX | **non** : la recherche web du 24/11/2026 n'a trouvé aucun dépôt ni aucune page à ces noms | rien |

Si ces cinq projets existent sous un autre nom ou dans un dépôt privé, il faut en donner
l'adresse pour qu'ils soient étudiés.

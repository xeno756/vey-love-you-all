# 07 — Series X : définition, graphismes au minimum, écran de lancement

Suite de [06-performances.md](06-performances.md). Mission : 10 images par seconde, graphismes
au plus bas et rendu en 720p, avec la version Series X du jeu seulement.

**Résumé honnête.** L'objectif de 10 images par seconde n'est pas atteint et rien de ce qui est
écrit ici n'a encore tourné avec PROJECT GAME : les lancements sont faits par l'utilisateur. Ce
document dit ce qui a été mesuré dans les journaux existants, ce qui a été ajouté, et dans quel
ordre l'essayer.

## 1. Ce que les lancements du 25/11 ont montré

| Lancement | Durée | Résultat |
|---|---|---|
| Series S, 23 h 55 | 7 min | **toujours bloqué sur le premier écran**, malgré la correction de la mémoire. Le jeu reçoit « 7 617 Mo utilisés sur 7 776 », dont 4 679 Mo de pages physiques qu'il détient lui-même. La cause du blocage reste inconnue. Piste mise de côté à la demande de l'utilisateur |
| Series X, 0 h 03, après `preparer` | 8 min | le monde arrive 53 s après le démarrage, aucun pipeline en attente (2 914 chargés du cache, 135 compilés), les dessins mesh indirects tournent (21 104 groupes par image). 0,5 à 1,7 image par seconde dans le monde |
| Series X, 0 h 17 | 2 min | même comportement ; c'est celui de la capture d'écran à 1 334,5 ms par image |

### Où partent les 1 334,5 ms (mesuré, lancement de 0 h 03, monde stable)

| Poste | Temps par image | Part |
|---|---|---|
| Attente de la carte | 650 à 1 130 ms | 65 à 85 % |
| Préparation par le processeur (copies de mémoire du jeu, textures, tampons) | 120 à 250 ms | 10 à 20 % |
| Recopie vers la mémoire du jeu après l'exécution | 50 à 110 ms | 5 à 8 % |
| Compilation de shaders | 0 ms | 0 % après `preparer` |
| Décodage vidéo | 0 ms | il n'y a pas de vidéo dans le monde |

La carte est occupée 74 à 87 % du temps. Les 2 152 dessins ne sont pas le poste principal : la
carte passe l'essentiel de son temps dans environ 1 450 appels de calcul par image (707 directs,
741 indirects), dont 16 passes plein écran en 2560 × 1440. À cela s'ajoute la mémoire vidéo : 6,7 Go demandés pour un budget
de 4,85 Go, donc une partie des textures vit dans la RAM et la carte y accède lentement.

Depuis `f9f130c`, chaque version du runtime (plus seulement la version diagnostic) écrit toutes
les cinq secondes une ligne `frame account` : ce qui limite l'image (la carte, la préparation,
le jeu), les parts en pourcentage, la mémoire vidéo et la RAM, les tailles de rendu et la taille
présentée.

## 2. Définition : ce qui est établi

| Point | Valeur mesurée (images capturées 1570 à 1648) |
|---|---|
| Rendu interne | **2560 × 1440** : 797 dessins et les 16 passes de calcul plein écran (57 600 groupes de 8 × 8) |
| Image présentée | **3840 × 2160** : 136 dessins (mise à l'échelle et interface), puis étirée dans la fenêtre |
| Définition dynamique | aucune : les tailles sont identiques d'une image à l'autre, à 1 image par seconde comme à 30 |
| Paramètre du jeu | aucun paramètre de définition dans l'exécutable. Le jeu choisit selon le type de console annoncé |
| `VICTORY_DISPLAY_MODE` | change le mode d'écran annoncé, sans effet sur le rendu interne (essai précédent) |

**Conclusion : en mode Series X, le jeu calcule en 1440p et présente en 4K, et le runtime ne
peut pas le lui faire changer par un réglage.** Réduire seulement les textures de rendu du côté
PC ne marche pas : les shaders de calcul du jeu adressent les pixels par leurs coordonnées
entières, pour 2560 × 1440. Ils n'écriraient qu'un coin de l'image.

Les deux seules voies vers un vrai 720p :

1. **Le profil Series S du jeu** (1280 × 720 interne, mesuré). Il reste bloqué sur son premier
   écran. C'est la voie la moins coûteuse, mais elle demande de reprendre ce blocage.
2. **Une mise à l'échelle par le runtime** : réécrire, à la traduction, les coordonnées de
   pixels dans des centaines de shaders de calcul, et réduire toutes les textures de rendu, les
   viewports et les constantes de taille que le jeu envoie. Estimation : plusieurs semaines de
   travail, avec un risque élevé d'images fausses. Non commencé.

Les pistes 960 × 540, 854 × 480 et 640 × 360 demandent la voie 2 : le jeu n'a aucun profil à ces
tailles.

Validation dans les journaux : la ligne `the title presents at this size` donne la taille
présentée à chaque changement, et les lignes `a size the title renders to or lets shaders write,
first asked for` donnent chaque taille de texture de rendu demandée. Un lancement en 720p réel
s'y lirait immédiatement.

## 3. Graphismes au minimum : ce qui a été ajouté

Rien n'est activé par défaut. Chaque réglage est une option du lanceur, isolée des autres.

### Textures réduites (`textures:N`, `VICTORY_TEXTURE_SKIP_MIPS`)

Les textures compressées du jeu sont copiées vers la carte sans leurs N premiers niveaux de
détail (N de 1 à 4), jamais en dessous de 128 pixels de côté. Un niveau en moins divise par 4
les octets gardés, décompactés et envoyés ; deux niveaux, par 16. Les vues du jeu sont refaites
sur les niveaux présents. Aucun modèle n'est touché : seule la netteté des textures baisse.

- Concerné : textures 2D compressées (BC1 à BC7) que le jeu remplit, 1 465 Mo de mémoire vidéo
  dans le dernier lancement.
- Non concerné : textures de rendu (3 455 Mo), textures 3D, formats non compressés.
- Une texture réduite que le jeu recopie ensuite sur la carte est refaite entière, et ses vues
  avec elle. La ligne `frame account` compte les textures réduites, les Mo épargnés et celles
  refaites entières.
- Tests : choix des textures, vues, et un test sur la carte (création réduite, retour à
  l'entier, vue refaite).

### Effets laissés de côté (`sans:groupe`, `VICTORY_SKIP_SHADERS`)

Le runtime peut ne pas exécuter les passes de calcul d'un effet, choisies par le nom de leurs
shaders. **Seuls des calculs sont concernés, jamais un dessin : aucune liste ne peut faire
disparaître un modèle.** Ce que la passe aurait écrit garde son contenu précédent, ce qui peut
assombrir ou figer l'effet.

Un groupe est un fichier texte dans `..\victory-profils`, hors du dépôt : un morceau de nom de
shader par ligne. Les noms viennent de la liste que `preparer` écrit à côté du cache
(`shader-names.txt`, 25 894 noms).

| Groupe | Ce qu'il retire | Part du calcul demandé | Risque |
|---|---|---|---|
| `particules` | mise à jour, tri et collisions des particules | 1,1 % et 330 appels indirects | faible |
| `vent` | simulation du vent | 4,1 % | faible |
| `flou-et-halos` | flou de mouvement, halos, reflets d'objectif | 1,5 % | faible à moyen |
| `reflets` | reflets en espace écran | 1,6 % | moyen |
| `lancer-de-rayons` | ombres lointaines et résolutions par matériau | 1,6 % et 200 appels indirects | moyen |
| `decals` | classement des décalcomanies | 3,2 % | moyen |
| `transparence` | lumière à travers les surfaces transparentes | 5,3 % | moyen à élevé |
| `volumetrie` | brouillard, nuages, diffusion | 15,4 % | élevé |
| `eclairage-indirect` | lumière rebondie | 21,8 % | élevé |

La part est celle des groupes de calcul demandés à la carte dans l'image capturée 1648. C'est un
travail demandé, pas un temps mesuré : seul le mode `profil` donne le temps par passe.

Laissés hors des groupes parce que l'image en dépend : l'anticrénelage temporel (15 %),
l'éclairage par tuiles (9 %), les masques d'ombre (6 %), le ciel, la pyramide de profondeur, le
tri des objets, l'animation des personnages, la profondeur de champ (elle fait partie de la
chaîne de l'image).

`minimal` regroupe `textures:2` et les quatre groupes à risque faible ou moyen : `particules`,
`vent`, `flou-et-halos`, `reflets`.

### Ce que ces réglages peuvent apporter

- Les quatre groupes de `minimal` : 8 % du calcul demandé. Tous les groupes : 56 %.
- Même si le temps de la carte baissait de moitié, l'image prendrait encore 0,6 à 0,8 s. La
  préparation et la recopie par le processeur prennent à elles seules 170 à 360 ms, plus que les
  100 ms d'une image à 10 par seconde.
- Les textures réduites visent surtout la mémoire vidéo. Si le dépassement du budget est bien ce
  qui ralentit la carte, le gain peut être plus grand que la part des textures ne le laisse
  penser. C'est une hypothèse, à lire dans le premier journal.

**10 images par seconde en mode Series X sur cette machine ne sont pas atteignables par ces
réglages.** Ils servent à gagner ce qui peut l'être et à mesurer.

## 4. Écran de lancement, cinématiques, sous-titres

| Observation | Ce que les journaux montrent |
|---|---|
| Écran de lancement lent | **vrai avant `preparer`, plus après.** Lancement du 24/11 à 22 h 11 : 2,9 puis 14,7, 22,6 et 25,3 images par seconde, pendant que 8 fils compilaient 1 200 à 1 600 pipelines. Lancements du 25/11 après `preparer` : 30 images par seconde dès la cinquième seconde. 30 est la cadence que le jeu fixe lui-même (intervalle de 33,3 ms) |
| Deux vidéos en même temps | **rien ne le montre.** Pendant l'introduction : 6 dessins par image, 1,3 Mo de texture envoyé par image (ce qui correspond à une seule image vidéo), 3,5 ms de carte, 10 % d'occupation |
| Décodage vidéo | fait par le code du jeu sur le processeur ; il tient ses 30 images par seconde. Ce n'est pas un poste de coût |
| Cinématique d'ouverture lente | **ce n'est pas une vidéo** : c'est de la 3D en temps réel (2 000 à 8 000 dessins par image dès l'arrivée du monde). Elle a le coût du jeu |
| Sous-titres lents | ils sont dessinés dans l'image du jeu (passe d'interface en 3840 × 2160) et avancent au rythme des images. Aucune cause propre |
| Premier affichage du monde | une image de 7,7 s à l'arrivée (10 176 dessins, 2,5 s de préparation), puis le régime décrit plus haut |

## 5. Ordre d'essai proposé

Chaque essai : deux à trois minutes dans le monde, au même endroit, puis fermer. Le journal de
chaque lancement porte ses options dans son nom.

| Étape | Commande | Ce qu'on lit |
|---|---|---|
| 1 | `lancer.cmd series-x` | la référence, avec les nouvelles lignes `frame account` |
| 2 | `lancer.cmd series-x textures:2` | mémoire vidéo utilisée, images par seconde, netteté |
| 3 | `lancer.cmd series-x minimal` | le gain des quatre groupes à faible risque ; l'image doit rester correcte |
| 4 | `lancer.cmd series-x minimal sans:eclairage-indirect` | le plus gros groupe, seul en plus |
| 5 | `lancer.cmd series-x minimal sans:volumetrie` | idem pour le brouillard |
| 6 | `lancer.cmd series-x profil` | (facultatif, plus lent) le temps de la carte passe par passe |

Si une image devient fausse ou noire à une étape, le groupe ajouté à cette étape est à retirer :
c'est l'intérêt de les essayer un par un.

`preparer` a été refait pour les deux versions du runtime compilées le 25/11. Il est à refaire
après chaque recompilation.

### Interrupteurs du fork, non essayés ici

Ils existent déjà dans le fork et se règlent avant d'appeler le lanceur, un seul à la fois :

| Variable | Effet attendu | Risque |
|---|---|---|
| `VICTORY_LATE_WAIT=1` | la carte travaille pendant que le processeur prépare l'envoi suivant, au lieu d'attendre | mode expérimental du fork, désactivé par défaut |
| `VICTORY_CARD_BUFFERS=0` | les copies de tampons du jeu quittent la mémoire vidéo (environ 490 Mo) | accès plus lents à ces tampons |

```powershell
$env:VICTORY_LATE_WAIT = "1"; project-one\lancer.cmd series-x
```

## 6. Ce qui n'a pas été fait

- Aucun essai en jeu de ce qui est décrit ici.
- Aucun temps de carte par passe : il faut un lancement `profil`.
- Pas de 720p réel en mode Series X (section 2).
- Le blocage du mode Series S n'est pas résolu.
- Les modes expérimentaux du fork (`VICTORY_LATE_WAIT`, `VICTORY_INPLACE`) ne sont pas essayés.
- Scarlett Layer, Xbox2Windows, TuxBox, ScarlettX et ZTRX : toujours introuvables, donc non
  étudiés.

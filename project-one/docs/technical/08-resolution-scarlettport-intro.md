# 08 — Définition réelle, ScarlettPort, cinématique d'introduction

Suite de [07-graphismes-minimaux.md](07-graphismes-minimaux.md). Mission du 25/11 : rendu réel
en 1920 × 1080, ScarlettPort comme référence pour les shaders, monde 3D au coût le plus bas,
cinématique d'introduction fluide.

**Rien de ce document n'a encore tourné avec PROJECT GAME.** Tout est compilé et testé hors jeu.
Chaque point dit quelle ligne du journal le prouvera ou le démentira au premier lancement.

## 1. Définition : le jeu calcule maintenant à la taille demandée

### Ce qui a été trouvé dans le jeu

Le document 07 concluait que le jeu fixe sa définition sans réglage possible. C'était faux sur
un point : le réglage existe, mais dans le code du jeu, pas dans ses paramètres.

| Élément | Ce que c'est | Où |
|---|---|---|
| Échelle de rendu | le rendu 3D est calculé à une part de l'image de sortie : 2/3 au minimum, d'où 2560 × 1440 pour une sortie en 3840 × 2160 | une constante du jeu (`0x28f1e7e`), trouvée par ScarlettPort |
| Taille de sortie | largeur et hauteur de l'image du jeu, rangées dans son objet de réglages d'affichage | positions `+0x204` et `+0x208` de cet objet |
| Accès aux réglages | une seule petite fonction, appelée par 89 endroits du jeu | `0x504c1b0` |
| Type de console | le jeu lit une valeur du registre, que le runtime lui fournit, et en déduit Series X ou Series S | `0x55deb88` |

Ces adresses valent pour la version du jeu installée ici (empreinte `b99f876b74d4a77f`). Chaque
modification vérifie d'abord l'empreinte et les octets attendus, et ne fait rien sinon.

### Ce que le runtime fait maintenant

| Variable | Option du lanceur | Effet |
|---|---|---|
| `VICTORY_OUTPUT_SIZE=1920x1080` | `sortie:1920x1080` | à chaque lecture de ses réglages, le jeu y trouve cette taille. Il dimensionne lui-même son image, son interface et ses textures de rendu pour elle |
| `VICTORY_RENDER_SCALE=1` | `echelle:1` | le rendu 3D est calculé à cette part de l'image (0,2 à 1) au lieu de 2/3 |
| les deux | `1080p` | image en 1920 × 1080 **et** rendu 3D en 1920 × 1080 |
| les deux | `720p`, `540p` | idem en 1280 × 720 et 960 × 540 |
| `VICTORY_NO_WATCHDOG=1` | par défaut (`chien-de-garde` pour le remettre) | le jeu ne se ferme plus lui-même quand son fil principal n'avance pas pendant 30 secondes |

Ce n'est pas une mise à l'échelle de la fenêtre : c'est le jeu qui calcule à cette taille. Ses
shaders, ses constantes et ses passes de calcul sont donc cohérents, ce qu'une réduction des
textures côté PC ne permettait pas.

Combinaisons utiles :

| Commande | Image du jeu | Rendu 3D | Pixels 3D par rapport à aujourd'hui |
|---|---|---|---|
| `lancer.cmd series-x` | 3840 × 2160 | 2560 × 1440 | 100 % |
| `lancer.cmd series-x echelle:0.5` | 3840 × 2160 | 1920 × 1080 | 56 % |
| `lancer.cmd series-x 1080p` | 1920 × 1080 | 1920 × 1080 | 56 %, et l'interface et la sortie en 1080p |
| `lancer.cmd series-x sortie:1920x1080` | 1920 × 1080 | 1280 × 720 | 25 % |
| `lancer.cmd series-x 720p` | 1280 × 720 | 1280 × 720 | 25 % |
| `lancer.cmd series-x 540p` | 960 × 540 | 960 × 540 | 14 % |

### Les preuves dans le journal

| Ligne | Ce qu'elle dit |
|---|---|
| `hook applied what="output size: the settings getter"` | la modification est en place (sinon `hook skipped`) |
| `resolution: the title chose its own size…` | la taille que le jeu avait choisie (demandée) et celle mise à la place (acceptée) |
| `display mode reported to the title width=… height=…` | le mode d'écran annoncé au jeu |
| `presenting to a window width=… height=…` | la taille du swap chain |
| `the title presents at this size` | la taille de l'image présentée (le backbuffer du jeu) |
| `a size the title renders to or lets shaders write, first asked for` | chaque taille de texture de rendu demandée |
| `frame account: … resolution: most draws in a viewport of WxH; draws by viewport size…` | **la taille à laquelle la 3D est réellement dessinée**, avec le nombre de dessins par taille |

Un lancement `1080p` réussi montre `presenting to a window width=1920 height=1080` et
`most draws in a viewport of 1920x1080`.

### Ce qui reste incertain

- Que le jeu accepte toutes ces tailles en mode Series X. Son profil Series S utilise bien
  1920 × 1080 par ces mêmes variables, ce qui rend la chose probable, pas certaine.
- Que l'échelle 1 convienne à son anticrénelage. ScarlettPort annonce la plage 0,2 à 1.
- Si l'image est fausse en `1080p`, essayer `sortie:1920x1080` seul, puis `echelle:0.5` seul :
  les deux leviers sont indépendants.

## 2. ScarlettPort

Dépôt lu en entier : <https://github.com/ZigZagBoi1/ScarlettPort> (cloné dans
`..\references\ScarlettPort`, rien n'en est exécuté).

### Ce que c'est

| Partie | Licence | Contenu |
|---|---|---|
| `runtime/victory` | GPL-3.0-or-later, comme PROJECT ONE | le fork de fxceless à `4e17478`, plus le fork victory-amd, des contributions de la communauté et leurs changements. **113 fichiers changés** par rapport à cette base |
| `runtime/xcl` | GPL-2.0-only | XCL modifié. **Aucune ligne ne peut entrer dans PROJECT ONE** |
| `scripts`, `docs`, `patches`, `launcher` | GPL-3.0-or-later | scripts PowerShell, lanceur, différences sous forme de patch |

PROJECT ONE part du même fork (`4e17478` est dans son historique). La pile VIctory de
ScarlettPort peut donc être fusionnée proprement, avec ses auteurs crédités.

Leur état annoncé : monde éclairé à 3 ou 4 images par seconde sur une RTX 3060, en profil
Series S. Ils exigent une carte avec mesh shaders. **PROJECT ONE a ce qu'ils n'ont pas** : le
remplacement des mesh shaders par du calcul, sans lequel la RX 5600 XT n'affiche rien.

### Ce que leur pile apporte

| Sujet | Chez eux | Chez nous aujourd'hui |
|---|---|---|
| Clé du cache de shaders | l'empreinte des sources du traducteur : une recompilation qui ne touche pas le traducteur garde le cache | l'empreinte de l'exécutable : chaque recompilation vide le cache |
| Pipelines graphiques avant le lancement | lus dans le cache de pipelines que le jeu livre (`pso_cache.rs`, `pso_state.rs`, `pipeline/precompiled.rs`) | seulement les pipelines de calcul et la moitié calcul des programmes mesh |
| Programmes trop chers pour le pilote AMD | laissés de côté par nom (`CS_RaytracingResolve` : environ 1 000 programmes de 5 à 9 s chacun) | compilés, ou sautés par nos listes |
| Mesh shaders de plus de 32 Kio | liés à leur pixel shader (`emit/link.rs`) | refusés |
| Corrections AMD (victory-amd) | vues de textures, dispatches de plus de 65 535 groupes découpés, indices de vues bornés | une partie seulement |
| Programmes à ondes de 64 | bascule par programme | bascule globale |
| Profil Series S | fonctionne, avec un module FidelityFX (`ffx.rs`) | bloqué sur le premier écran |
| Hooks du jeu | échelle de rendu, upscaler, chien de garde | **repris** (section 1) |
| Vitesse | une douzaine d'interrupteurs non mesurés (`VICTORY_SYNC_PRECISE`, `VICTORY_KNOWN_WRITES`, `VICTORY_REBAR`…) | absents |

### Décision : fusionner leur pile VIctory, ne pas garder deux systèmes

La préparation des shaders de PROJECT ONE devient celle de ScarlettPort (clé par sources du
traducteur, pipelines graphiques précompilés depuis le cache du jeu, programmes trop chers
laissés de côté), et notre commande `prepare` reste l'outil qui la déclenche avant le lancement.
Ce qui est propre à cette carte (mesh shaders en calcul) reste et s'y ajoute.

La fusion est **faite sur la branche `project-one/scarlettport`** (dossier de travail
`..\victory-scarlettport`, avec son propre cache) :

- le patch `patches/victory-stack.patch` s'applique en fusion à trois voies : 49 fichiers sans
  conflit, 42 fichiers nouveaux, 20 fichiers en conflit, tous résolus ;
- le traducteur de shaders est **celui de ScarlettPort tel quel**, avec nos seuls changements
  réappliqués dessus (les programmes mesh en calcul) ;
- une seule commande de préparation, `prepare`, qui sait aussi compiler les pipelines graphiques
  que le cache de pipelines du jeu liste (`--graphics-pipelines`, repris de leur
  `precompile-graphics`) ; leur commande `warm`, qui faisait une partie de la nôtre, est retirée ;
- **752 tests réussis, 0 échec**, lint propre, politique du dépôt respectée.

Elle n'a **pas encore tourné avec PROJECT GAME**. Tant que ce n'est pas fait, la branche stable
reste `project-one/integration`. Le détail des résolutions et la façon de l'essayer sont dans
[REPRISE.md](../../REPRISE.md).

### Compatibilité avec la RX 5600 XT

| Point | Constat |
|---|---|
| Mesh shaders | absents sur RDNA1. ScarlettPort les exige ; notre remplacement par calcul est gardé |
| Ondes | le traducteur de leur pile lit la plus grande onde de la carte (`host_max_wave`) et l'inclut dans la clé du cache : un cache fait sur NVIDIA n'est jamais servi à une AMD |
| Shader Model | 6.6 demandé par eux à cause des mesh shaders ; nos chemins de calcul n'en dépendent pas |
| Pilote AMD | leur pile laisse de côté les programmes qui bloquent le compilateur AMD : à reprendre tel quel |

## 3. La cinématique d'introduction

### Cause trouvée dans le code

Le jeu règle sa cadence sur une horloge du runtime : un top toutes les 33,3 ms pour 30 images
par seconde. Cette horloge n'est pas celle de l'écran. Et le runtime présentait chaque image
avec `Present(0)` : l'image part à l'écran au prochain rafraîchissement, en remplaçant celle
qui attendait encore.

Deux horloges proches, des présentations en avance ou en retard d'une ou deux millisecondes :
une image est affichée pendant un rafraîchissement, la suivante pendant trois, certaines
jamais. Le compteur affiche bien 30 images par seconde, mais un film qui bouge de façon
continue saccade comme si deux films étaient superposés. Un écran fixe, comme le logo, ne le
montre pas.

Ce que les journaux existants établissent par ailleurs :

- un seul flux vidéo en apparence : 6 dessins et environ 1,4 Mo de données envoyés par image ;
- le jeu décode le film sur le processeur avec sa propre bibliothèque (Bink 2), sur plusieurs
  fils, et la carte n'est occupée qu'à 10 % ;
- la résolution de l'horloge de Windows est déjà réglée à 1 ms par le runtime.

**Ce n'est pas mesuré en jeu** : c'est la lecture du code. Les mesures ci-dessous le
confirmeront ou non.

### Correction

Chaque image est maintenant présentée pour autant de rafraîchissements que l'intervalle du jeu
en contient : deux sur un écran à 60 Hz pour 30 images par seconde, une pour 60. Les images se
suivent alors à intervalles égaux. Ce nombre ne dépasse jamais ce que l'intervalle contient,
pour que l'écran ne ralentisse pas l'horloge du jeu. `sans-synchro`
(`VICTORY_PRESENT_SYNC=0`) remet l'ancien comportement pour comparer.

Le film, sa synchronisation et son son ne sont pas touchés : rien n'est sauté ni remplacé.

### Mesures ajoutées

Dans la ligne `frame account`, toutes les cinq secondes :

| Mesure | Texte dans le journal |
|---|---|
| Images présentées | `presents: N since the last line` |
| Écart entre deux présentations | `… ms apart on average (shortest …, longest …)` |
| Images perdues | `… came early (a frame lost unless synchronised)` |
| Images répétées | `… late (the frame before shown again)` |
| Temps de présentation | `the present call took … ms on average, … at most` |
| Cadence réelle | `each frame shown for N refreshes of a … Hz display` |
| Attentes processeur et carte, temps de copie | parts déjà présentes dans la même ligne |

Et une ligne `video:` toutes les deux secondes de lecture, grâce à un compteur placé devant
les fonctions de la bibliothèque vidéo du jeu :

| Mesure | Texte dans le journal |
|---|---|
| Lecteurs vidéo actifs | `video: N players open (M at once at most…)` |
| Images du film montrées et cadence | `… frames shown in … s (… a second)` |
| Images sautées par le film pour retard | `… the library asked to skip for lateness` |
| Temps de décodage par image | `… frames decoded, … ms each on average, … ms at most` |

Si deux films jouent réellement en même temps, la première mesure le dira.

## 4. Monde 3D au coût le plus bas

| Levier | Option | État |
|---|---|---|
| Moins de pixels 3D | `1080p`, `720p`, `540p`, `echelle:` | nouveau, le plus fort : les passes de calcul plein écran suivent la taille du rendu |
| Textures grossières | `textures:6 plancher:32` | étendu : jusqu'à 6 niveaux en moins, plancher réglable de 32 à 1024 pixels |
| Effets laissés de côté | `sans:…`, `minimal` | inchangé (document 07) |
| Modèles, animations | — | jamais touchés par ces options |

Mesuré le 25/11 sur votre lancement `minimal` : 27 à 50 % de temps d'image en moins pendant les
trois premières minutes du monde, rien de mesurable ensuite ; mémoire vidéo toujours au-dessus
du budget (6,2 Go pour 4,8), à cause des textures de rendu en 1440p. La définition est donc le
levier à essayer en premier : elle réduit ces textures.

## 5. Ordre d'essai

| Étape | Commande | À regarder |
|---|---|---|
| 1 | `lancer.cmd series-x` | l'introduction est-elle fluide ? Lignes `presents:` et `video:` |
| 2 | `lancer.cmd series-x 1080p` | l'image est-elle correcte ? `most draws in a viewport of 1920x1080` |
| 3 | `lancer.cmd series-x 720p` | même chose en 1280 × 720, images par seconde |
| 4 | `lancer.cmd series-x 720p minimal` | avec les textures et effets réduits |
| 5 | `lancer.cmd series-x 540p textures:6 plancher:32 minimal` | le plus bas disponible |

Si une étape échoue (image fausse, écran noir, fermeture), le journal de ce lancement suffit
pour comprendre : son nom porte les options.

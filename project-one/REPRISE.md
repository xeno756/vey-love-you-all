# Reprise du travail

Où en est PROJECT ONE et comment continuer, si une session s'arrête en cours de route.
Mis à jour le 25/11/2026.

**Au 27/11 à 12 h 15** : **Series S** : `meilleure.cmd series-s performance` (scarlettport
`5056699`). Le jeu prend enfin son vrai profil Lockhart (bug S-005 corrigé). Réglages propres à
la Series S :
- fenêtres de 1 Mio ;
- limite de recopie de 1 Mio ;
- agrandisseur du jeu.

Mesures : 4,59 images/s, médiane 208 ms, prison atteinte, image propre. Mission Series S en
cours : `..\series-s\` (registre des bugs, audit). La Series X reste
`meilleure.cmd series-x performance`, inchangée.

**Au 27/11 à 10 h 30** : **version à donner scarlettport `1f0cbd4`**, même commande
(`meilleure.cmd series-x performance`), shaders déjà prêts. Depuis `d86412a` :
- la course des textures est corrigée (`d2e2383`, détail dans le CHANGELOG). Sous charge, plus
  aucune perte de carte en 26 min, dont le réglage qui la perdait avant ;
- les cases ne se lisent plus comme des zéros quand le jeu change de texture (`1f0cbd4`) : fini
  le visage rouge et la serviette noire d'une image ;
- la rotation BC7 de BCPack est corrigée.

Partie de 12 min : 4,07 images/s, carte gardée. Outils de rafales dans
`..\victory-logs\outils\` : `rafale.ps1`, `clignote.py`, `rafales-banc.ps1`. Suite :
- le chargement plus rapide en début de scène (4 fils, rattrapage plus long), maintenant sûr
  à essayer ;
- la limite de recopie de scarlet-fps (vitesse) ;
- l'accès mémoire par adresse pour le lancer de rayons.

**Au 27/11 à 8 h 30** : **version à donner scarlettport `d86412a`**, même commande, shaders
retraduits (`meilleure.cmd series-x performance preparer` déjà fait). Ajoute à `145bde0` les
correctifs de traduction d'avatarmanz, sans celui des cubes. Validations de 15 min : 4,36
images/s, carte gardée (8 h 15) ; une perte de carte à 3,5 min (8 h 04, adresse basse,
famille textures). Le lancer de rayons du jeu (`VICTORY_BVH_BUILDER=1`) fait planter le jeu
au chargement. Suite :
- la course des textures aux adresses basses ;
- les visages des premiers plans de scène ;
- l'accès mémoire par adresse pour le lancer de rayons ;
- la préparation des envois sur un fil à nous (vitesse).

**Au 27/11 à 7 h 30** : **version à donner scarlettport `145bde0`**, même commande. Par rapport à
5 h 45 :
- tout le rendu est gardé hors lancer de rayons (pas de DXR sur la carte) et reflets en espace
  écran (cassés : yeux bleu-blanc, murs violets) ;
- les vues de textures orphelines sont remises en file (visages des PNJ).

15 min dans le monde avec promenade : 4,72 images/s, 95 % sous 280 ms, ni plantage ni perte de
carte. Point de restauration `restore/2026-11-27-0550`. Diagnostics et captures dans
`..\diagnostics\`. Les touches du runtime (`VICTORY_PRESS_FILE`, `LS_UP`…) font jouer le
personnage : scripts `..\victory-logs\outils\promenade.ps1` et `promenade2.ps1`.

**Au 27/11 à 5 h 45** : **version à donner scarlettport `906f7f0`**, commande inchangée
`meilleure.cmd series-x performance`. Elle ajoute trois réglages par défaut :
- la carte gardée éveillée ;
- la lumière indirecte du jeu gardée, avec l'estimation d'avatarmanz à la place de sa sortie ;
- 20 s de rattrapage des textures au début des scènes.

15 min dans le monde : 5,04 images/s, médiane 192 ms, 95 % sous 229 ms, ni plantage ni perte de
carte. La qualité progresse : pièces éclairées par le rebond, visages texturés plus tôt. Restent
des visages sans texture aux premiers plans d'une scène (un rouge, un noir).

Suite :
- la course des textures, qui perd la carte dès qu'on en charge plus vite (adresses hors de tout
  objet) ;
- le plantage du jeu `title+0x4dbd74d` ;
- la préparation des envois sur un fil à nous.

**Au 27/11 à 5 h** : version à donner inchangée (commande `meilleure.cmd series-x performance`,
5,05 images/s). scarlettport `271fc42` n'y change rien par défaut, en partie à vérifier (5 h). Le
matin a porté depuis avatarmanz/VIctory-7900XTX le rangement des pages du port n°1, la commande
indirecte « seule » et les copies de tables du port n°2. Le port n°1 s'ouvre maintenant
(`VICTORY_PORT_CONSTANTS=1`). Leur exécutable exige les mesh shaders et ne tourne pas sur la
RX 5600 XT.

Mesures (5 min dans le monde chacune ; le banc s'arrête à 300 s, `-Monde 1200` pour 20 min) :
- port n°1 + `vitesse:2` + étage 3 : 155-175 ms, mais le jeu plante (`0x4dbd74d`, `0x34bce9`) ;
- port n°1 + `VICTORY_INPLACE=1` à l'étage 2 : 179 ms, puis le même plantage dans la partie
  suivante ;
- `VICTORY_INPLACE=1` seul : carte perdue ;
- port n°2 : plus lent.

Le plantage du jeu `title+0x4dbd74d` est lu dans son code (doc 14, fin) : le contexte principal
enregistre une transition d'état de ressource. Le faire revenir déplace le plantage
(`VICTORY_MAIN_CONTEXT_SLOT=skip`). Il sort plus souvent avec le port n°1.

Suite :
- trouver pourquoi le jeu demande une transition sur le contexte principal (la date
  `[ctx+0x12194]`, l'état par case de `0x144dbe690`) ;
- la préparation des envois sur un fil à nous (`VICTORY_ASYNC_SUBMIT` d'avatarmanz,
  `queue/behind.rs`, 103 → 75 ms chez eux), qui demande l'étage 3 ;
- la recopie à la demande : protéger les pages que la carte a écrites et ne recopier qu'à la
  lecture du jeu, pour un `vitesse:2` sans données périmées.

**Au 27/11 à 2 h 35** : **version à donner `6f3bdc8`** (branche fusionnée), commande
inchangée `meilleure.cmd series-x performance` : **5,05 images/s, médiane 193 ms, 99 % sous
277 ms, 20 min sans perte de carte** (partie de 2 h 10, sans aucune variable). Ce qui a changé :
le relais s'arrête au niveau 2 (la présentation attend que tout ait tourné) au lieu du niveau 3
(présentation envoyée en avance) : toutes les pertes de carte des réglages rapides venaient de
là (niveau 3 : pertes à 3, 4, 4, 6 min ; niveau 2 : rien en 15 et 20 min ; relais coupé : rien
en 16 min mais 3,78 images/s). Avec ça la synchronisation des descripteurs passe sur 4 fils et
pendant la préparation des listes. Pourquoi la présentation en avance perd la carte n'est pas
trouvé ; `VICTORY_PRESENT_LAG=1` (retard de la carte borné à une image au niveau 3) en essai.

**Au 26/11 à 22 h 15 (heure locale ; l'entrée suivante dit 22 h 35 par erreur, elle date de
20 h 50)** : version à donner `f5fc552` (branche fusionnée) : un verrou par groupe de 16 cases
du tas de vues ; 4,91 images/s, médiane 200 ms, 10 min sans perte de carte. Plantage
`title+0x4dbd74d` expliqué (doc 14, réglages où le jeu prend de l'avance seulement). Pertes de
carte des variantes rapides : aucune faute côté processeur selon la couche de débogage ;
adresses dans aucun objet connu. Puis `c9cd850` : la synchronisation sur 4 fils par défaut, 5,29
images/s, médiane 183 ms, carte gardée (22 h 11) ; **mais la partie de 20 min de 22 h 47 a
perdu la carte vers la 14e minute** (5,4 images/s jusque-là, adresse dans aucun objet connu) :
pas encore livrable tel quel. Partie de 20 min avec `VICTORY_SYNC_THREADS=1` (le niveau de
`f5fc552`) à 23 h 20 : 4,95 images/s, médiane 200 ms, carte gardée. **Version à donner :
`b9764f5`** (un fil par défaut ; `VICTORY_SYNC_THREADS=4` et `VICTORY_SYNC_AHEAD=1` en option ;
`bfa2b73` n'y change que du diagnostic). Restent : le plantage du jeu `title+0x4dbd74d` (vu une
fois en neuf parties par défaut, 23 h 32 ; le prendre comme un cache vide déplace le plantage) ;
la cinématique d'ouverture surexposée (presque blanche) sans les halos, sombre avec visages noirs
avec eux (`VICTORY_KEEP_BLOOM=1`, 4,71 images/s, carte gardée 7 min) ; plus de temps pour les
textures (`VICTORY_TEXTURE_SPAN_MS=400`) texture les visages mais perd la carte (3,5 min).
Planches : `..\victory-logs\captures\planche-ctx400.png`, `planche-halos.png`. Suite : l'attente du fil d'envoi sur les
remplissages de constantes (45 ms par image ; le journal note leur mémoire) et la recopie (80 ms).

**Au 26/11 à 22 h 35 (fin de soirée de travail seul)** : version stable de la branche
fusionnée `e7a7f9e` : 4,41 images/s, médiane 224 ms, 10 min sans perte de carte. Rapport
complet : [docs/technical/14-nuit-26-11.md](docs/technical/14-nuit-26-11.md). Objectifs (15
images/s constantes, cinématique parfaite) non atteints : une course du runtime fait perdre la
carte dès qu'on accélère ; les visages noirs viennent du suivi du streaming des textures.
Suite : la couche de débogage de Direct3D sur la variante rapide (en cours), puis le port #6
(verrou des gardes).

**Au 26/11 à 19 h 30** : version stable par défaut (marquage éteint) ~5 images/s ;
`VICTORY_TABLE_SLOTS=1` donne 6,3 à 7 images/s mais 4 pertes de carte en 16 min (objets
libérés encore lus, cause non trouvée) : en option seulement (`39f8fca`). Port #1 impossible
tel quel (tas de 174 Mo en 3 vues hors ordre de section ; Direct3D refuse 2 vues en un tas).
`vitesse:1` : rien. Visages noirs en cinématique : la lumière indirecte écartée (garbage
magenta si gardée, `VICTORY_KEEP_INDIRECT=1`) ; `VICTORY_AMBIENT_FILL=0.15` rend la cellule
belle mais les visages restent noirs piquetés de blanc en extérieur : capture de diagnostic
en cours (`..ictory-captures`, cibles de rendu des images 40-140 après le monde).

**Au 26/11 à 18 h 45 (travail de nuit, seul, l'utilisateur dort)** : objectif fixé par
l'utilisateur : 15 images/s stables, jamais sous 10, cinématique et monde parfaits, qualité
gardée. Je lance moi-même `banc.ps1` (autorisé depuis 17 h 25). Fait sur la branche
fusionnée : `3052ba9` (cases de table notées, lecture des cases faites sans verrou, table des
vues en lecture partagée ; 6,4 à 7 images/s contre 5) ; trois pertes de la carte venaient du
marquage des vues de tampons à adresse (les programmes qui les prennent par index lisent la
table d'adresses par case) : corrigé en les excluant (non encore commité). Port #1
(`VICTORY_PORT_CONSTANTS=1`, constantes sur place) écrit dans `port_constants.rs`, éteint,
à mesurer ensuite pour la recopie (70 à 90 ms par image). Captures de la fenêtre :
`..ictory-logs\outils\capture-fenetre.ps1`.

**Au 26/11 à 17 h 20** : le port #2 a donné 5,1 images par seconde à 16 h 41, mais avec des
descripteurs périmés et la carte perdue à 14 min : retiré de `performance` (`255c2ba`, mot
`marques`). La mémoire du PC est pleine (BLOCKERS 44). Suite : une partie sans `marques`
avec Discord et Edge fermés, la réponse de l'utilisateur sur le profil Series S, puis le
port #1.

**Au 26/11 à 16 h 40** : port #2 d'avatarmanz repris (`ea371e3`, branche fusionnée) : le
jeu marque les cases de descripteurs qu'il écrit, la synchronisation des tas (110 ms par
image à 14 h 56) ne relit plus que celles-là. Dans `performance`, jamais lancé. Suite : la
partie de l'utilisateur (`meilleure.cmd series-x performance`), puis le port #1
(DECISIONS 49, BLOCKERS 42).

**Au 26/11 à 14 h** : 6,43 images par seconde mesurées (13 h 14) ; les murs manquaient : deux
correctifs de l'amont repris (`a5b2dd3`), non vérifiés ; `performance` prend `attentes-textures`.

**Au 26/11 à 13 h** : le monde s'affiche (lancement de l'utilisateur à 11 h 12, 5,6 images
par seconde). Fait depuis, non vérifié en jeu : visages noirs (`84fe251`), ombres du soleil
proches (`b3dd536`), essai `attentes-textures` (`3018756`). Tout est dans
[docs/technical/12-audit-moteur-26-11.md](docs/technical/12-audit-moteur-26-11.md). Point
de restauration : `project-one/avant-moteur-26-11`. Suite : les lancements de l'utilisateur,
puis la fusion de l'amont fxceless (98 commits).

**Au 26/11 à 11 h 15** : la version à essayer est `meilleure.cmd series-x performance`
(branche fusionnée, `2cf979c`), avec le brouillard rendu de nouveau : sans lui le monde
était noir de 8 h 25 à 10 h 38, et les vitesses mesurées alors ne comptent pas. Le monde
n'est pas encore vérifié en jeu. L'option `prereglage:bas` (préréglage SafeMode abaissé)
n'a jamais tourné. Je ne lance plus le jeu : l'utilisateur fait les lancements
(DECISIONS 35). Suite : sa capture du monde, puis la mission vitesse (pics de temps
d'image, attentes de la carte), voir [LOT_LEDGER.md](LOT_LEDGER.md).

## Branches

| Branche ou étiquette | Rôle |
|---|---|
| `project-one/integration` | **branche stable**, celle que le lanceur utilise. Tout ce qui y est commité compile et passe les tests |
| `project-one/before-scarlettport` | étiquette : l'état avant la mission du 25/11 (retour arrière possible) |
| `project-one/scarlettport` | la pile VIctory de ScarlettPort fusionnée avec PROJECT ONE, dans le dossier `..\victory-scarlettport`. Essayée en jeu le 25/11 : introduction fluide, puis carte perdue à la première image du monde (erreur de fusion, corrigée à 9 h 30) ; depuis, le monde s'affiche (lancement de 9 h 41) |

## La meilleure version aujourd'hui

La branche fusionnée, depuis que le lancement de 9 h 41 a montré le monde. Mesures de ce
lancement et profils `rapide` et `maximum` qui en découlent :
[docs/technical/10-premier-lancement-reussi-et-profils.md](docs/technical/10-premier-lancement-reussi-et-profils.md).
Prochain essai conseillé : `project-one/meilleure.cmd series-x rapide`.

Depuis le 25/11 à 11 h 20, le travail suit des lots numérotés (mesure, streaming,
figement, barycentriques, niveau de détail…) : [PERF_JOURNAL.md](PERF_JOURNAL.md) pour
chaque commit et sa mesure, [DECISIONS.md](DECISIONS.md) pour les choix faits sans
question, [BLOCKERS.md](BLOCKERS.md) pour ce qui attend. Les lots 0 à 2 sont écrits et
testés hors jeu (763 tests) ; les lots suivants attendent un lancement qui montre le
monde chargé.

Le 25/11 au soir, le fork VIctory-7900XTX (remote git `x7900`, clone dans
`..\references\VIctory-7900XTX`) a été lu et trois de ses correctifs essayés en jeu : deux
sont retirés, le troisième est coupé par défaut. Ce qui reste à reprendre, et dans quel
ordre : [docs/technical/11-victory-7900xtx.md](docs/technical/11-victory-7900xtx.md).
Après tout changement du traducteur : `meilleure.cmd series-x preparer`, puis
`preparer-graphiques` (le lancement de 18 h 41, préparé ainsi, a eu les meilleurs 95e et
99e centiles de la journée).

| Version | Commande | Ce qui est vu en jeu |
|---|---|---|
| Branche stable | `project-one/lancer.cmd series-x` | entre dans le monde et y tourne, modèles 3D visibles, 0,3 à 2,5 images par seconde pendant les 25 secondes de monde du lancement de 8 h 20 ; introduction saccadée à ce lancement, corrigée depuis sans nouvel essai |
| Branche fusionnée | `project-one/meilleure.cmd series-x` | introduction fluide ; carte perdue à la première image du monde aux lancements de 8 h 26, 8 h 59 et 9 h 00 ; cause corrigée (`30cba67`) ; lancement de 9 h 41 : le monde s'affiche, 0,2 à 3,1 images par seconde, arrêt par manque de mémoire après 7 min 35 s |

Causes et preuves : [docs/technical/09-plantages-du-25-11.md](docs/technical/09-plantages-du-25-11.md).

Si un lancement se ferme encore, chercher dans son journal, dans cet ordre : `resolution: a
check of the title's own…` (une vérification du jeu franchie, avec sa position), `first-chance
exception` (l'endroit du plantage), `host GPU device removed` (la carte, avec ce qui était en
cours).

## Fait le 25/11 sur `project-one/integration`

| Commit | Contenu |
|---|---|
| `ebfaa24` | définition : hooks de ScarlettPort (échelle de rendu, upscaler, chien de garde) et taille de sortie du jeu (`VICTORY_OUTPUT_SIZE`) ; dessins comptés par taille de viewport |
| `ec60820` | présentations calées sur l'écran ; compteur du lecteur vidéo du jeu |
| `6dad160` | textures jusqu'à 6 niveaux en moins, plancher réglable |
| `fc73e53` | les vérifications internes du jeu ne l'arrêtent plus quand sa taille d'image est imposée (plantage de `1080p` et `720p`) |
| `c6e9908` | le jeu apprend quand ses images ont été affichées (introduction saccadée) |

Détail et mesures attendues : [docs/technical/08-resolution-scarlettport-intro.md](docs/technical/08-resolution-scarlettport-intro.md).

Aucun de ces changements n'a encore tourné avec PROJECT GAME.

## La fusion ScarlettPort : faite, à essayer en jeu

La pile VIctory de ScarlettPort (`fx-harvest` à `38e8e7c`, GPL-3.0-or-later) est fusionnée dans
PROJECT ONE sur la branche `project-one/scarlettport`. Elle vit dans son propre dossier de
travail, `..\victory-scarlettport`, avec sa propre configuration (`victory.toml`) et son propre
cache (`%LOCALAPPDATA%\victory\cache-scarlettport`) : elle et la branche stable ne se vident pas
leurs caches l'une l'autre.

| Commit de la branche | Contenu |
|---|---|
| `7a8a970` | la fusion : 113 fichiers de ScarlettPort, 20 conflits résolus, nos fonctions gardées |
| `60bd4da` | une seule commande de préparation (`prepare`), qui peut aussi compiler les pipelines graphiques du cache du jeu |
| `1fdb0bf` | mélange à deux sources gardé sur la première cible seulement : 1 868 pipelines de plus acceptés par la carte |
| `da761fb` | le programme que le pilote AMD ne finit pas de compiler n'est plus compilé (ce n'était pas la cause de la carte perdue) |
| `30cba67` | **l'erreur de fusion qui perdait la carte au monde** : le tampon mesh ne partage plus l'emplacement 252 avec la table que 94 % des shaders lisent ; la clé du cache de shaders est la même en préparation et en jeu |

État à `30cba67` : compile, **756 tests réussis, 0 échec**, lint propre avec et sans la fonction
`diagnostics`, politique du dépôt respectée. La correction de 9 h 30 a tourné avec le jeu
au lancement de 9 h 41 : le monde s'affiche.

### Le piège de cette fusion, à ne pas refaire

Notre code et celui de ScarlettPort réservent chacun des numéros : emplacements de shader
(251 à 255), codes des mots calculés par le runtime (`0x100` et suivants), clés par liste.
Deux numéros identiques ne donnent aucune erreur de compilation ni de test : le premier test
rencontré gagne et l'autre fonction disparaît en silence. Avant toute nouvelle fusion de leur
arbre : comparer ces numéros des deux côtés, et comparer chaque fichier fusionné à **leur**
fichier (pas seulement au nôtre) pour voir ce que nos ajouts placent devant leur code.
L'emplacement du tampon mesh est maintenant vérifié à la compilation.

### Comment les conflits ont été résolus

| Fichier | Résolution |
|---|---|
| Manifestes | les deux bibliothèques de décompression (`zlib-rs` de notre base, `miniz_oxide` de leurs fichiers) |
| `shader_cache.rs` | leur clé par sources du traducteur, dans nos dossiers par version |
| `pipeline/make.rs` | leur file (pipelines laissés de côté, limite des fils « early ») avec notre ordre « demandé en dernier, fait en premier » |
| `pipeline.rs` | notre `MeshVariant` et leur `linked` côte à côte |
| `pipeline/host.rs` | notre fichier, avec leurs 22 changements reportés un par un (script `..\references\merge_pipeline_host.py`) : étage géométrie, test de bornes de profondeur, mélange désactivé sur les cibles entières, mesh liés au pixel shader |
| `descriptor/host.rs` | leur création du tas de vues (carte du tas, borne AMD de victory-amd), notre remise à jour des vues de textures réduites adaptée à leur structure |
| `texture.rs` | leurs envois partiels ; une texture réduite ne lit plus que la mémoire du jeu des niveaux gardés |
| `draw.rs`, `host.rs`, `indirect.rs`, `resource.rs` | les deux côtés gardés ; leur budget d'envoi de textures par image |
| Traducteur (`victory-shader`) | **le leur, tel quel**, avec nos seuls changements réappliqués dessus : les programmes mesh en calcul |

### Ce qui a été laissé de côté, et pourquoi

- Les fichiers d'essai faits d'enregistrements du cache de pipelines du jeu, et leurs tests :
  rien qui vienne du jeu n'est gardé dans ce dépôt.
- L'option Agility SDK pour Windows 10 : cette machine est sous Windows 11.
- Un test de l'étage géométrie, ignoré : il échoue aussi dans l'arbre de ScarlettPort sur cette
  machine (les validateurs DXIL d'ici refusent le programme du test).
- Tout `runtime/xcl` (GPL-2.0 seule).

### Ce qui reste à faire sur cette branche

1. Un lancement par l'utilisateur avec le profil `rapide`, pour mesurer ce qu'il change
   (document 10). Pour savoir quels pipelines manquent aux dessins laissés de côté :
   `meilleure.cmd series-x rapide profil` (version diagnostic).
2. Les 403 pipelines refusés faute de barycentriques sur la RX 5600 XT : faire porter les poids
   des trois coins par une sortie interpolée du shader de sommets généré pour les programmes
   mesh, et lire cette sortie dans le pixel shader à la place de `SV_Barycentrics`.
3. Mesurer `pipelines-du-jeu` (pipelines graphiques précompilés servis au jeu) : compilations
   restantes pendant le chargement du monde.
4. Essayer un par un les interrupteurs de vitesse de ScarlettPort (liste dans leur
   `docs/RUNTIMES.md`), aucun n'est mesuré.

### Pour l'essayer

```powershell
cd "..\victory-scarlettport"
cargo build --release -p victory
project-one\lancer.cmd series-x preparer
project-one\lancer.cmd series-x
```

Ses journaux portent `-scarlettport` dans leur nom. `preparer-graphiques` compile en plus les
pipelines graphiques que le cache du jeu liste (long).

### Pour la faire entrer dans la branche stable

Seulement après un lancement réussi par l'utilisateur :

```powershell
git checkout project-one/integration
git merge project-one/scarlettport
```

## Règles à garder

- Lancer PROJECT GAME : permis de nouveau depuis le 26/11 à 8 h 55 (DECISIONS 29), par
  `banc.ps1`, un lancement à la fois, jamais pendant une compilation, sans fermer les
  programmes de l'utilisateur. L'interdiction du matin du 26/11 est levée par sa réponse ;
  si une consigne plus récente la rétablit, redemander avant le lancement suivant.
- Travailler seul : aucun sous-agent, aucune délégation.
- Une seule version à lui donner à essayer : `series-x performance`, avec tous les
  correctifs (DECISIONS 30).
- Ne jamais fermer un processus par son nom : le jeu tourne sous le nom `victory.exe`.
- Aucune ligne de `runtime/xcl` (GPL-2.0 seule) dans ce dépôt.
- Rien qui vienne du jeu dans le dépôt : journaux, captures, listes de noms de shaders et
  scripts d'analyse restent dans les dossiers voisins (`..\victory-logs`, `..\victory-captures`,
  `..\victory-profils`).
- Après chaque recompilation : `lancer.cmd series-x preparer` pour la version normale, et
  `lancer.cmd series-x profil preparer` pour la version diagnostic.

## Outils d'analyse de l'exécutable

Un petit désassembleur a servi à trouver les adresses de la section 1 du document 08. Il est
hors du dépôt (dossier temporaire de la session) et se refait en quelques minutes avec la
bibliothèque `iced-x86` : lecture d'une plage par adresse, recherche d'une constante, recherche
des appels vers une adresse. Les adresses trouvées et la façon de les retrouver sont dans le
document 08 ; rien du jeu n'est dans le dépôt.

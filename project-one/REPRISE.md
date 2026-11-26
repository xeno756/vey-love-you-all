# Reprise du travail

Où en est PROJECT ONE et comment continuer, si une session s'arrête en cours de route.
Mis à jour le 25/11/2026.

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

- Ne jamais lancer PROJECT GAME : l'utilisateur fait tous les lancements.
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

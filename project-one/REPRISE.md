# Reprise du travail

Où en est PROJECT ONE et comment continuer, si une session s'arrête en cours de route.
Mis à jour le 25/11/2026.

## Branches

| Branche ou étiquette | Rôle |
|---|---|
| `project-one/integration` | **branche stable**, celle que le lanceur utilise. Tout ce qui y est commité compile et passe les tests |
| `project-one/before-scarlettport` | étiquette : l'état avant la mission du 25/11 (retour arrière possible) |
| `project-one/scarlettport` | branche de la fusion de la pile VIctory de ScarlettPort. **Fusion non terminée**, rien n'y est commité au-delà de `integration` |

## Fait le 25/11 sur `project-one/integration`

| Commit | Contenu |
|---|---|
| `ebfaa24` | définition : hooks de ScarlettPort (échelle de rendu, upscaler, chien de garde) et taille de sortie du jeu (`VICTORY_OUTPUT_SIZE`) ; dessins comptés par taille de viewport |
| `ec60820` | présentations calées sur l'écran ; compteur du lecteur vidéo du jeu |
| `6dad160` | textures jusqu'à 6 niveaux en moins, plancher réglable |

Détail et mesures attendues : [docs/technical/08-resolution-scarlettport-intro.md](docs/technical/08-resolution-scarlettport-intro.md).

Aucun de ces changements n'a encore tourné avec PROJECT GAME.

## La fusion ScarlettPort : où elle en est

Source : `..\references\ScarlettPort` (clone du dépôt), patch
`patches\victory-stack.patch` = leur pile VIctory par rapport à `4e17478`, qui est dans notre
historique.

### Pour la reprendre

```powershell
git worktree add ..\victory-scarlettport project-one/scarlettport
cd ..\victory-scarlettport
git merge project-one/integration        # la branche part d'un état plus ancien
git apply --3way ..\references\ScarlettPort\patches\victory-stack.patch
git status --short                       # UU = conflits, A = fichiers nouveaux
```

Puis recopier par-dessus les fichiers déjà résolus, gardés dans
`..\references\merge-resolved` avec leur chemin :

| Fichier | Comment il a été résolu |
|---|---|
| `Cargo.toml` et ceux de `victory-game`, `-gpu`, `-hle`, `-shader` | les deux bibliothèques de décompression (`zlib-rs` de notre base, `miniz_oxide` de leurs fichiers) |
| `apps/victory/src/run.rs` | les deux lignes gardées |
| `crates/victory-game/src/sgd5.rs` | leur fonction `unpack` partagée, avec notre décompression |
| `crates/victory-gpu/src/shader_cache.rs` | leur clé par sources du traducteur, dans nos dossiers par version (`cache()` au lieu de `CACHE.get()`) |
| `crates/victory-gpu/src/pipeline/make.rs` | leur file (pipelines laissés de côté, limite des fils « early ») avec notre ordre « demandé en dernier, fait en premier » |
| `crates/victory-gpu/src/pipeline/library.rs` | notre commentaire (même correction des deux côtés) |
| `crates/victory-gpu/src/pipeline.rs` | notre `MeshVariant` et leur `linked` côte à côte ; `Translated` a un champ `writes` de plus |

Attention : ces fichiers ont été résolus par rapport à `94d46c1`. Les commits faits depuis sur
`integration` touchent `hooks.rs`, `windows.rs`, `lib.rs` du chargeur (leur `hooks.rs` est repris
tel quel, plus notre taille de sortie : garder **le nôtre**), `draw.rs`, `host.rs`, `frame.rs`,
`resource.rs` et ajoutent `pacing.rs` et `video.rs`.

### Ce qui reste à résoudre

| Fichier | Conflits | Nature |
|---|---|---|
| `crates/victory-gpu/src/pipeline/host.rs` | 6 | le plus délicat. Nous avons sorti la construction de la signature racine dans `root_for` (avec `Variant` pour les mesh en calcul) ; eux ont modifié ce même code en place : étage géométrie, test de bornes de profondeur, `host_blend`, mesh liés au pixel shader, `SLOT_WORDS`, `from_precompiled`. Partir de **notre** fichier et y reporter leurs 22 changements un par un |
| `crates/victory-gpu/src/descriptor/host.rs` | 4 | leur table des mots de descripteurs et leur carte du tas, contre nos vues de textures réduites |
| `crates/victory-gpu/src/texture.rs` | 4 | leurs réécritures des verrous et du détuilage par bandes, contre notre décalage de niveaux |
| `crates/victory-gpu/src/draw.rs` | 3 | leurs fenêtres d'indirects et barrières, contre nos dispatches mesh |
| `crates/victory-gpu/src/host.rs` | 3 | compteurs et attentes |
| `crates/victory-gpu/src/resource.rs` | 2 | |
| `crates/victory-gpu/src/indirect.rs` | 1 | leur découpage des gros dispatches contre notre conversion mesh |
| `crates/victory-shader/src/emit/{lower,mod,shader,wave,valu_tests}.rs` | 18 | traducteur : leurs ondes de 64 par programme contre les changements de notre base |
| `Cargo.lock` | 1 | à régénérer par `cargo build` |

Une fois les conflits résolus : `cargo build --workspace --all-targets`, corriger, puis les
tests (`VICTORY_DXIL` doit pointer sur le `dxil.dll` de DXC 1.9), `cargo clippy`, `cargo xtask
policy`. Vérifier en particulier que les tests du remplacement des mesh shaders passent
toujours : c'est ce que ScarlettPort n'a pas et dont la RX 5600 XT dépend.

Ne fusionner dans `integration` qu'après un lancement réussi par l'utilisateur.

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

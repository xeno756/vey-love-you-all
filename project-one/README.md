# PROJECT ONE

Runtime personnel pour Windows 11, destiné à exécuter PROJECT GAME (une version Xbox Series
dont l'utilisateur possède les fichiers) sur **cette** machine : Ryzen 7 5700, 16 Go de RAM,
Radeon RX 5600 XT 6 Go.

PROJECT ONE est la branche `project-one/integration` du fork de VIctory par fxceless
(GPL-3.0-or-later). Ce n'est pas une fusion de XCL et de VIctory : les deux licences sont
incompatibles (voir [COMPATIBILITY_REPORT.md](COMPATIBILITY_REPORT.md)) et leurs architectures
n'ont pas d'interfaces communes (voir [ARCHITECTURE.md](ARCHITECTURE.md)). XCL sert de
référence : ses bonnes idées sont réécrites ici, jamais copiées.

## État réel

**Non jouable sur cette machine à ce jour.** Le jeu démarre, charge le monde et affiche le
générique d'ouverture, à environ 1 image par seconde en mode Series X (mesuré le 25/11/2026 :
0,5 à 1,7). Le remplacement des mesh shaders fonctionne en jeu, dessins directs et indirects.
Le mode Series S reste bloqué sur son premier écran. Les graphismes réduits (textures, effets
laissés de côté) sont écrits et testés hors jeu, pas encore essayés en jeu.

- Pourquoi 1 image par seconde, et ce qui a été changé :
  [docs/technical/06-performances.md](docs/technical/06-performances.md).
- Définition, graphismes au minimum, écran de lancement, ordre d'essai :
  [docs/technical/07-graphismes-minimaux.md](docs/technical/07-graphismes-minimaux.md).
- Définition réelle en 1080p, ScarlettPort, cinématique d'introduction :
  [docs/technical/08-resolution-scarlettport-intro.md](docs/technical/08-resolution-scarlettport-intro.md).
- Les plantages du 25/11 et leurs causes :
  [docs/technical/09-plantages-du-25-11.md](docs/technical/09-plantages-du-25-11.md).
- Où en est le travail et comment le reprendre : [REPRISE.md](REPRISE.md).
- Le remplacement des mesh shaders :
  [docs/technical/04-shaders-ecran-noir.md](docs/technical/04-shaders-ecran-noir.md), section 3 bis.
- Le reste : [KNOWN_ISSUES.md](KNOWN_ISSUES.md).

Ce qui a changé par rapport au fork : [CHANGELOG.md](CHANGELOG.md).

## Compiler

Prérequis : Windows 10/11 x64, Rust (la version est fixée par `rust-toolchain.toml` et
s'installe toute seule avec rustup), les outils de compilation C++ de Visual Studio, une carte
Direct3D 12 avec le shader model 6.6.

```powershell
cargo build --release -p victory
```

Le programme est `target\release\victory.exe`. Le fichier de réglages est celui de VIctory
(`victory.example.toml` → `victory.toml`, ou `VICTORY_CONFIG` vers un fichier existant).

## Lancer les tests

Ils ne lancent jamais PROJECT GAME et n'ont besoin d'aucun de ses fichiers.

```powershell
$env:VICTORY_DXIL = "$env:LOCALAPPDATA\Programs\DXC\v1.9.2609\bin\x64\dxil.dll"
cargo test --workspace --no-fail-fast -- --test-threads=4
cargo clippy -p victory-gpu -p victory-loader --all-targets -- -D warnings
```

`VICTORY_DXIL` désigne le validateur DXIL de DXC 1.9. Sans lui, les tests prennent celui du
SDK Windows 10.0.22621 (DXIL 1.6), trop ancien : deux tests `emit::rect` échouent alors sans
que le code soit en cause. Ce validateur ne sert qu'aux tests.

Résultats mesurés : [TEST_RESULTS.md](TEST_RESULTS.md).

## Documents

| Fichier | Contenu |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | modules, origine de chacun, décisions |
| [HARDWARE_PROFILE.md](HARDWARE_PROFILE.md) | la machine, mesurée |
| [OPTIMIZATION_REPORT.md](OPTIMIZATION_REPORT.md) | ce qui a été optimisé, mesuré ou non |
| [COMPATIBILITY_REPORT.md](COMPATIBILITY_REPORT.md) | licences, dépendances, fonctions de la carte |
| [TEST_RESULTS.md](TEST_RESULTS.md) | avant / après, commandes exactes |
| [KNOWN_ISSUES.md](KNOWN_ISSUES.md) | ce qui ne marche pas, et pourquoi |
| [CHANGELOG.md](CHANGELOG.md) | les commits de PROJECT ONE |
| [docs/technical/](docs/technical/) | rapports détaillés et décisions |

Rien de PROJECT GAME n'est stocké ici (voir `docs/asset-policy.md` à la racine du dépôt).

## Lancer

`project-one\lancer.cmd` lance PROJECT GAME avec le runtime compilé dans ce dépôt
(`target\release\victory.exe`), en Series X par défaut.

| Commande | Effet |
|---|---|
| `lancer.cmd` | Series X |
| `lancer.cmd series-s` | Series S : moins de mémoire et une définition interne plus basse, le mode le plus proche de cette machine |
| `lancer.cmd series-x preparer` | **ne lance pas le jeu** : traduit ses shaders dans le cache et compile ses pipelines de calcul et ceux de ses programmes mesh (environ 10 minutes). À refaire après chaque recompilation du runtime |
| `lancer.cmd series-x capture` | version diagnostic (`target\diag\victory.exe`) : enregistre des images de l'écran et des tampons internes du jeu (normales, profondeur, couleurs) dans `..\victory-captures`, hors du dépôt |
| `lancer.cmd series-x profil` | version diagnostic : mesure le temps de la carte commande par commande (plus lent) |
| `lancer.cmd series-x test` | montre ce qui serait lancé, sans rien lancer |
| `lancer.cmd series-x 1080p` | **définition réelle** : le jeu calcule son image et son rendu 3D en 1920 × 1080 (`720p` : 1280 × 720, `540p` : 960 × 540) |
| `lancer.cmd series-x sortie:1920x1080` | taille de l'image du jeu seule ; son rendu 3D en fait les deux tiers |
| `lancer.cmd series-x echelle:0.5` | part de l'image à laquelle la 3D est calculée (0,2 à 1) |
| `lancer.cmd series-x sans-synchro` | présente les images sans les caler sur l'écran, comme avant (pour comparer) |
| `lancer.cmd series-x chien-de-garde` | laisse le jeu se fermer lui-même après 30 secondes sans avancer (désarmé par défaut) |
| `lancer.cmd series-x textures:2` | textures du jeu sans leurs 2 premiers niveaux de détail (1 à 6) : moins de mémoire vidéo, textures plus floues. `plancher:32` descend jusqu'à 32 pixels de côté |
| `lancer.cmd series-x sans:vent+particules` | laisse de côté les passes de calcul de groupes d'effets (fichiers `..\victory-profils\<groupe>.txt`) ; jamais un dessin |
| `lancer.cmd series-x minimal` | `textures:2` et les groupes à faible risque : `particules`, `vent`, `flou-et-halos`, `reflets` |

`series-x` et `series-s` se remplacent l'un l'autre dans chaque commande. `capture`, `profil`,
`test`, `textures:N`, `sans:…` et `minimal` se combinent. Les graphismes réduits et leur ordre
d'essai sont décrits dans [docs/technical/07-graphismes-minimaux.md](docs/technical/07-graphismes-minimaux.md). `preparer` suit la version choisie : la version diagnostic a son propre
cache de shaders.

Le dossier du jeu vient de `victory.toml`, à la racine du dépôt (non suivi par git). Chaque
lancement garde son journal dans `..\victory-logs`, hors du dépôt ; le dernier est aussi copié
dans `%TEMP%\victory-err.log`.

Les modes `capture` et `profil` ralentissent le jeu : pour juger de la fluidité, utiliser la
version normale.

La version diagnostic se compile avec
`cargo build --profile diag -p victory --features diagnostics`. Les tampons internes ne sont
enregistrés que pour quelques images : la 2e, la 6e, la 30e et la 80e après la première image
de 500 dessins ou plus.

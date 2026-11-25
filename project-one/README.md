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

**Non jouable sur cette machine à ce jour.** La carte n'a pas de mesh shaders, et toute la
géométrie du monde de PROJECT GAME en dépend. Le remplacement est maintenant écrit pour les
dessins directs, mais il n'a pas encore été essayé en jeu. Les dessins indirects restent
sautés. Détails : [docs/technical/04-shaders-ecran-noir.md](docs/technical/04-shaders-ecran-noir.md),
section 3 bis. Le reste est dans [KNOWN_ISSUES.md](KNOWN_ISSUES.md).

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
| `lancer.cmd series-s` | Series S (reste sur un écran noir sur cette machine) |
| `lancer.cmd series-x capture` | version diagnostic (`target\diag\victory.exe`) : enregistre des images de l'écran et des tampons internes du jeu (normales, profondeur, couleurs) dans `..\victory-captures`, hors du dépôt |
| `lancer.cmd series-x test` | montre ce qui serait lancé, sans rien lancer |

Le dossier du jeu vient de `victory.toml`, à la racine du dépôt (non suivi par git). Le journal
va dans `%TEMP%\victory-err.log`.

La version diagnostic se compile avec
`cargo build --profile diag -p victory --features diagnostics`. Les tampons internes ne sont
enregistrés que pour quelques images : la 2e, la 6e, la 30e et la 80e après la première image
de 500 dessins ou plus.

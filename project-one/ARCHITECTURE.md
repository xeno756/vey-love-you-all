# Architecture de PROJECT ONE

## Décision de départ

| Option | Verdict | Pourquoi |
|---|---|---|
| Fusionner XCL et VIctory module par module | **écartée** | XCL est GPL-2.0-only, VIctory GPL-3.0-or-later : du code des deux ne peut pas former une œuvre unique distribuable. Leurs modules ne partagent aucune interface : mémoire (XCL : fenêtres « UMA » en mémoire vidéo ; VIctory : mémoire du jeu en RAM + copies sur la carte), D3D12.X, traduction des shaders (XCL : HLSL compilé par DXC ; VIctory : DXIL écrit directement). Un « loader de l'un + GPU de l'autre » obligerait à tout réécrire. |
| Partir de XCL | écartée | Sur cette machine : manque de mémoire vidéo à 80 s ou réinitialisation à 259 s en Series X ; jamais de chargement du monde en Series S ; aucune géométrie à mesh shaders dessinée, sur aucune carte. |
| **Partir du fork de VIctory par fxceless et y réécrire les bonnes idées de XCL** | **retenue** | C'est le seul des trois projets (VIctory, XCL, Leonida) où la cinématique et le prologue s'affichent (sur RTX 4070). Il est GPL-3.0-or-later, et l'audit le juge meilleur que XCL dans chaque module pour cette machine (`docs/technical/01-audit-fork-xcl.md`). |

Leonida (GPL-3.0-or-later, Vulkan) a été examiné : le jeu n'y démarre pas, et son dépôt
GitHub n'est plus accessible. Son idée de mesh shaders en calcul sert de référence de
conception pour le remplacement prévu, sans reprise de code.

## Modules

Les douze modules demandés correspondent aux crates existantes. Aucune n'a été réécrite ;
chacune reste remplaçable et testable seule (`cargo test -p <crate>`).

| Module demandé | Crate / dossier | Taille | Origine | Modifié par PROJECT ONE |
|---|---|---|---|---|
| Runtime | `apps/victory` (`run`) | — | fork | non |
| Loader | `crates/victory-loader`, `crates/victory-pe` | 2 868 + 758 lignes | fork | oui : code venu de XCL retiré (licence) |
| Compatibilité noyau / services | `crates/victory-hle` (kernel, xgameruntime, xmem, xuser…) | 18 163 | fork | non |
| Gestionnaire mémoire | `victory-hle` `xmem` + `victory-gpu` `resource`, `shadow`, `host` | — | fork | oui : mémoire de transfert bornée, plancher de textures adapté |
| Backend GPU | `crates/victory-gpu` | 37 067 | fork | oui : format R9G9B9E5, rapport de réinitialisation, pipelines mesh en calcul + dessin |
| Compilateur de shaders | `crates/victory-shader` | 42 177 | fork | oui : programmes mesh en calcul, vertex shader généré (`emit/mesh_compute.rs`) |
| Textures | `crates/victory-formats` (tiling, BCPack) + `victory-gpu` `texture` | 4 365 | fork | non |
| Ressources | `victory-gpu` `resource`, `descriptor` | — | fork | oui (voir GPU) |
| Système de fichiers / streaming | `victory-hle` `files`, `dstorage` | — | fork | non |
| Diagnostics | `victory-loader` `crash`, `victory-gpu` `host` (rapports), journaux `tracing` | — | fork | oui : profil machine au démarrage, rapport de réinitialisation |
| Configuration | `crates/victory-core` (`victory.toml`, variables `VICTORY_*`) | 424 | fork | non |
| Tests | tests unitaires et d'intégration de chaque crate, `xtask` (politique des fichiers) | — | fork | oui : 9 tests ajoutés |

## Règles de PROJECT ONE

1. **Aucun code de XCL.** Les idées de XCL sont décrites, puis réécrites (voir
   `COMPATIBILITY_REPORT.md`).
2. **Rien de PROJECT GAME dans le dépôt** : la politique `docs/asset-policy.md` du dépôt
   s'applique (`cargo xtask policy`).
3. **Changement ciblé plutôt que réécriture.** Chaque commit a une raison mesurée, écrite
   dans son message.
4. **Suivre le fork.** La branche est rebasée sur `origin/main` du fork (dernière base :
   `a803c87`), pour profiter de ses corrections et garder des modifications faciles à proposer
   à son auteur.

## Ce qu'il manque pour cette machine

Dans l'ordre d'impact (détails dans `docs/technical/`) :

1. **Fin du remplacement des mesh shaders.** Les dessins directs sont écrits (étapes 1, 2
   et 4). Il reste les dessins indirects, la comparaison sur WARP, le regroupement des dessins
   et l'essai en jeu.
2. **Limite de boucles et découpage du travail de calcul**, pour rester sous les 2 s de
   Windows sur une carte plus lente que la RTX 4070 de l'auteur.
3. **Partage des pages de mémoire vidéo entre render targets**, avec plafond (idée de XCL).
4. **Compilation de pipelines limitée selon la mémoire libre** (idée de XCL).

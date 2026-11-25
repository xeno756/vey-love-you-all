# Résultats des tests (24/11/2026, sur cette machine)

Aucun test ne lance PROJECT GAME. Les tests « GPU » tournent sur la RX 5600 XT, ou sur WARP
quand ils le demandent.

## Commandes

```powershell
$env:VICTORY_DXIL = "$env:LOCALAPPDATA\Programs\DXC\v1.9.2609\bin\x64\dxil.dll"   # sauf pour la 1re mesure
cargo build --workspace --all-targets --locked
cargo test --workspace --no-fail-fast --locked -- --test-threads=4
cargo clippy -p victory-gpu -p victory-loader --all-targets --locked -- -D warnings
```

Toolchain Rust 1.98.1, fixée par `rust-toolchain.toml`, en mode debug pour les tests.

## Résumé

| Mesure | Code | Compilation | Réussis | Échecs | Ignorés |
|---|---|---|---|---|---|
| 1. Fork tel que cloné, sans `VICTORY_DXIL` | `7b0f736` | 111 s, 0 avertissement | 481 | 3 | 7 |
| 2. Fork à jour, sans nos changements | `a803c87` | 88 s | 504 | 0 | 8 |
| 3. **PROJECT ONE** | `a803c87` + 3 commits | 38 s (incrémentale), 0 avertissement | **511** | **0** | 8 |

- **Clippy** (`-D warnings`) sur les crates modifiées : aucune erreur.
- **Version de production** : `cargo build --release -p victory` réussit.

### Les 3 échecs de la mesure 1

| Test | Cause | Réglé par |
|---|---|---|
| `pipeline::host::tests::a_mesh_pipeline_loads_from_the_library` | carte sans mesh shaders (`E_INVALIDARG`) | le fork lui-même (`3930989`) |
| `emit::rect::tests::rectangles_are_drawn_whole` | validateur DXIL 1.6 du SDK Windows trop ancien | `VICTORY_DXIL` vers DXC 1.9 |
| `emit::rect::tests::the_rectangle_stage_is_a_geometry_shader` | même cause | même chose |

## Par module (mesure 2 → mesure 3)

| Binaire de test | Fork à jour | PROJECT ONE | Écart |
|---|---|---|---|
| victory (app) | 1 | 1 | — |
| victory_core | 11 | 11 | — |
| victory_formats | 24 (+4 ignorés) | 24 (+4 ignorés) | — |
| victory_game | 3 | 3 | — |
| **victory_gpu** | 112 (+1 ignoré) | **121** (+1 ignoré) | **+9 tests ajoutés** |
| victory_hle | 96 | 96 | — |
| no_dll_planting | 1 | 1 | — |
| **victory_loader** | 24 | **22** | −2 : tests du code repris de XCL, retirés avec lui |
| load_probe | 2 | 2 | — |
| title_dlls | 1 | 1 | — |
| victory_pe | 7 | 7 | — |
| victory_pipeline | 4 | 4 | — |
| victory_shader | 214 (+3 ignorés) | 214 (+3 ignorés) | — |
| xtask | 4 | 4 | — |

## Les 9 tests ajoutés (victory-gpu)

| Test | Vérifie | Sur la carte ? |
|---|---|---|
| `upload_memory_is_made_within_nine_tenths_of_the_shared_budget` | la règle de place de la mémoire de transfert | non (calcul pur) |
| `a_wait_lasts_at_most_the_wait_and_none_while_starved` | l'attente bornée à 2 s | non |
| `upload_memory_for_a_list_is_kept_or_made` | réserve ou création d'un bloc | oui |
| `spare::clearing_takes_everything_kept` | libération de la réserve | non |
| `the_texture_floor_is_a_quarter_of_the_card_at_most_2_gib` | plancher des textures | non |
| `the_host_is_asked_for_the_series_s_r9g9b9e5_target` | refus du pilote et tailles mémoire | oui |
| `an_r9g9b9e5_target_gets_a_format_the_host_renders_to` | texture, vues, format de pipeline, effacement relu | oui |
| `r9g9b9e5_views_of_a_target_take_its_stand_in` | vues UAV et de lecture | oui |
| `the_removal_report_says_how_long_the_card_had_it` | rapport de perte de la carte | non |

Les trois tests du format R9G9B9E5 échouent si on retire la correction : vérifié par l'agent 3.

## Ce que ces tests ne prouvent pas

- **Le rendu en jeu** : un test unitaire réussi ne prouve pas que PROJECT GAME s'affiche.
- **La mémoire réelle en jeu** : elle ne se mesure qu'en lançant le jeu.
- **Les tests de charge** : aucun test de charge GPU ou mémoire reproductible n'existe encore.
  À ajouter avec le remplacement des mesh shaders (comparaisons sur WARP).

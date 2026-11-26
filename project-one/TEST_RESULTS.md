# Résultats des tests (24 et 25/11/2026, sur cette machine)

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
| 3. PROJECT ONE | `a803c87` + 3 commits | 38 s (incrémentale), 0 avertissement | 511 | 0 | 8 |
| 4. PROJECT ONE avec le remplacement des mesh shaders | `a803c87` + 5 commits de code | incrémentale, 0 avertissement | 520 | 0 | 8 |
| 5. PROJECT ONE après la session performances | `a803c87` + 12 commits de code | incrémentale, 0 avertissement | 526 | 0 | 9 (dont le banc d'essai, lancé à la main) |
| 6. **PROJECT ONE avec les graphismes réduits (25/11)** | `5db5360` | incrémentale, 0 avertissement | **532** | **0** | 9 |

| 7. **PROJECT ONE avec la définition réelle et l'introduction (25/11)** | `6dad160` | incrémentale, 0 avertissement | **542** | **0** | 9 |
| 8. Branche `project-one/scarlettport` (fusion ScarlettPort) | `60bd4da` | 0 avertissement | **752** | **0** | 12 |
| 9. Branche stable après les corrections du 25/11 au matin | `c6e9908` | 0 avertissement | **545** | **0** | 9 |
| 10. **Meilleure version** (branche fusionnée corrigée) | `4c88086` | 0 avertissement | **756** | **0** | 12 |
| 11. Branche fusionnée après la correction de l'erreur de fusion (9 h 30) | `30cba67` | 0 avertissement | **756** | **0** | 12 |

Mesure 7 : un test de cadencement du fork (`waits_end_on_ticks_and_count_frames`, des tops de
5 ms) a échoué une fois pendant qu'une préparation de shaders occupait tous les cœurs ; relancé
seul, il passe trois fois sur trois. Ce n'est pas un test ajouté par PROJECT ONE.

Mesure 8 : 752 tests dont ceux de ScarlettPort. Un test de l'étage géométrie y est ignoré, il
échoue aussi dans l'arbre de ScarlettPort sur cette machine. Les tests qui lisaient des
enregistrements du cache de pipelines du jeu ont été retirés avec leurs fichiers.

### Les 10 tests ajoutés pour la définition et l'introduction (25/11)

| Test | Module | Vérifie | Sur la carte ? |
|---|---|---|---|
| `scales_outside_the_floor_are_refused`, `half_scale_patches_both_upscalers`, `odd_scales_patch_only_the_own_upscaler` | victory-loader (repris de ScarlettPort) | les modifications de l'échelle de rendu | non |
| `output_sizes_are_read_as_width_by_height` | victory-loader | lecture de `VICTORY_OUTPUT_SIZE` : 640×360 à 3840×2160, pair | non |
| `the_settings_getter_is_sent_to_the_hook` | victory-loader | les 16 octets écrits à l'entrée de la fonction du jeu | non |
| `the_hook_puts_the_size_asked_for_in_the_settings` | victory-loader | le hook écrit la taille demandée aux deux positions, et rien d'autre | non |
| `the_titles_video_calls_are_counted_on_their_way` | victory-loader | chaque appel vidéo atteint la bibliothèque avec ses arguments ; lecteurs, images, sauts et décodages comptés | non |
| `a_frame_is_shown_for_the_refreshes_its_interval_holds` | victory-gpu | 2 rafraîchissements à 60 Hz pour 30 images par seconde, jamais plus que l'intervalle | non |
| `display_modes_named_59_run_at_59_94` | victory-gpu | fréquences d'écran | non |
| `presents_off_the_titles_interval_are_counted` | victory-gpu | présentations en avance et en retard | non |

### Les 6 tests ajoutés le 25/11

| Test | Module | Vérifie | Sur la carte ? |
|---|---|---|---|
| `a_frame_is_put_down_to_what_took_half_of_it` | victory-gpu | ce que la ligne `frame account` désigne comme limite d'une image | non |
| `only_compressed_textures_with_levels_to_spare_are_lowered` | victory-gpu | quelles textures sont réduites et de combien : compressées, 2D, jamais sous 128 pixels, toujours en blocs entiers, jamais une texture de rendu | non |
| `views_of_a_lowered_texture_take_the_levels_it_has` | victory-gpu | les vues du jeu sont refaites sur les niveaux présents | non |
| `a_lowered_texture_is_made_whole_for_a_copy` | victory-gpu | une texture réduite redevient entière quand le jeu la recopie, et la vue qui la montrait est refaite | **oui** |
| `only_compute_pipelines_of_listed_shaders_are_left_out` | victory-gpu | une liste ne laisse de côté que des pipelines de calcul, jamais un dessin | non |
| `entries_name_shaders_by_hash_or_by_part_of_their_name` | victory | lecture des listes de groupes : noms partiels, numéros, commentaires | non |

- **Clippy** (`-D warnings`) sur tout le dépôt (`cargo clippy --workspace --all-targets`), et
  avec la fonction `diagnostics` pour `victory`, `victory-gpu` et `victory-hle` : aucune erreur.
- **Version de production** (`cargo build --release -p victory`) et **version diagnostic**
  (`cargo build --profile diag -p victory --features diagnostics`) : compilées.

### Les 3 échecs de la mesure 1

| Test | Cause | Réglé par |
|---|---|---|
| `pipeline::host::tests::a_mesh_pipeline_loads_from_the_library` | carte sans mesh shaders (`E_INVALIDARG`) | le fork lui-même (`3930989`) |
| `emit::rect::tests::rectangles_are_drawn_whole` | validateur DXIL 1.6 du SDK Windows trop ancien | `VICTORY_DXIL` vers DXC 1.9 |
| `emit::rect::tests::the_rectangle_stage_is_a_geometry_shader` | même cause | même chose |

## Par module (mesure 2 → mesure 6)

| Binaire de test | Fork à jour | PROJECT ONE | Écart |
|---|---|---|---|
| **victory (app)** | 1 | **2** | **+1 test ajouté** (25/11) |
| victory_core | 11 | 11 | — |
| victory_formats | 24 (+4 ignorés) | 24 (+4 ignorés) | — |
| victory_game | 3 | 3 | — |
| **victory_gpu** | 112 (+1 ignoré) | **132** (+2 ignorés) | **+20 tests ajoutés** (15 le 24/11, 5 le 25/11), +1 banc d'essai |
| **victory_hle** | 96 | **98** | **+2 tests ajoutés** |
| no_dll_planting | 1 | 1 | — |
| **victory_loader** | 24 | **22** | −2 : tests du code repris de XCL, retirés avec lui |
| load_probe | 2 | 2 | — |
| title_dlls | 1 | 1 | — |
| victory_pe | 7 | 7 | — |
| victory_pipeline | 4 | 4 | — |
| **victory_shader** | 214 (+3 ignorés) | **221** (+3 ignorés) | **+7 tests ajoutés** |
| xtask | 4 | 4 | — |

## Les 15 tests ajoutés (victory-gpu)

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
| `the_mesh_buffers_are_one_for_every_list` | un seul tampon d'enregistrements mesh et un seul tampon d'arguments, quel que soit le demandeur | oui |
| `the_mesh_record_buffer_is_32_mib_by_default` | taille par défaut du tampon | non |
| `mesh_arguments_are_a_dispatch_and_a_draw_of_the_groups_that_fit` | dessins mesh indirects : la passe de conversion écrit les arguments du calcul et du dessin, bornés à la place disponible, vides pour un enregistrement vide | oui |
| `the_pipeline_asked_for_last_is_made_first` | ordre de compilation des pipelines : celui demandé par l'image la plus récente d'abord | non |
| `a_preparation_leaves_what_is_no_shader_alone` | la préparation ignore ce qui n'est pas un shader | oui (rien à préparer) |
| `old_builds_translations_are_removed` | cache de shaders : un dossier par version, les versions périmées et l'ancien rangement sont retirés | non |

## Les 2 tests ajoutés (victory-hle)

| Test | Vérifie | Sur la carte ? |
|---|---|---|
| `title_use_counts_what_the_title_committed` | la mémoire dite au jeu suit ce qu'il engage, libère et rend dans ses propres réservations | non |
| `working_set_statistics_report_the_titles_own_memory` | la réponse est la mémoire du jeu, pas celle de tout le processus | non |

## Bancs d'essai et mesures hors jeu

| Mesure | Commande | Résultat sur cette machine |
|---|---|---|
| Schéma de dessin mesh, un par un contre regroupé | `cargo test -p victory-gpu --release bench -- --ignored --nocapture` | 256 dessins : 1,8 à 2,7 ms un par un, 0,95 à 1,3 ms regroupés ; 2 304 dessins : 5,4 ms contre 1,1 ms |
| Traduction de tous les shaders du jeu, sans cache | `victory translate <archive des shaders> --jobs 12` | 26 084 programmes en 2 min 40 s |
| Préparation, cache vide | `victory prepare --compute-pipelines` | 25 466 shaders traduits et 2 723 pipelines de calcul en 9 min 41 s ; 3 min 21 s quand les pipelines sont déjà dans la bibliothèque |
| Préparation des programmes mesh | `victory prepare --mesh-pipelines` | 8 762 programmes mesh traduits en calcul, 8 762 pipelines acceptés par la carte, 0 refusé, en 2 min 25 s |
| Préparation complète, pipelines déjà dans la bibliothèque (25/11) | `victory prepare --compute-pipelines --mesh-pipelines` | 25 563 étapes : 25 467 traduites, 96 non traduisibles ; 11 486 pipelines dans la bibliothèque, 0 refusé ; 3 min 06 s |

Détails et lecture : [docs/technical/06-performances.md](docs/technical/06-performances.md).

## Les 7 tests ajoutés (victory-shader, remplacement des mesh shaders)

| Test | Vérifie | Sur la carte ? |
|---|---|---|
| `a_mesh_program_lowers_to_a_compute_shader_with_its_record_slot` | un programme mesh devient un compute shader valide, avec son UAV `u252` | non (validateur DXIL) |
| `a_mesh_program_as_compute_writes_its_records` | le calcul écrit les sommets et les triangles de deux groupes, au bon endroit | oui |
| `a_record_holds_the_vertices_then_the_primitive_words` | disposition d'un enregistrement | non |
| `draws_and_buffers_count_whole_groups` | nombre de sommets dessinés, groupes par tampon | non |
| `primitive_words_pack_three_indices_or_cull` | mot de triangle (trois indices, ou « éliminé ») | non |
| `the_vertex_shader_validates_and_declares_the_mesh_outputs` | le vertex shader généré est valide et déclare les sorties du programme mesh | non (validateur DXIL) |
| `the_vertex_shader_draws_a_record_and_skips_culled_primitives` | le vertex shader dessine un enregistrement avec un vrai pixel shader et saute le triangle éliminé | oui |

Les trois tests du format R9G9B9E5 échouent si on retire la correction : vérifié par l'agent 3.

## Mission 10 à 15 images par seconde, cycle 1 (26/11, branche fusionnée)

Commandes, dans `..\victory-scarlettport` :

```powershell
$env:VICTORY_DXIL = "$env:LOCALAPPDATA\Programs\DXC\v1.9.2609\bin\x64\dxil.dll"
cargo test --release --workspace --no-fail-fast
cargo clippy --release --workspace --all-targets
cargo xtask policy
```

| Vérification | Résultat | Durée |
|---|---|---|
| Suite complète | 554 tests réussis, 0 échec, 10 ignorés ; la suite de `victory-gpu` s'est arrêtée sur une violation d'accès (code `0xc0000005`) | 57 s |
| `victory-gpu` repassée à part, trois fois | 226 réussis, 0 échec, 3 ignorés, à chaque fois | 4 à 5 s |
| `victory-gpu` en boucle | 50 passes réussies sur 55 ; 5 échecs sur deux tests intermittents | |
| La même boucle sans les changements du cycle | 24 passes réussies sur 30 ; 6 échecs sur les deux mêmes tests | |
| `clippy` | aucun avertissement | |
| `xtask policy` | 242 fichiers, rien à signaler | |
| Version diagnostic | compilée | 41 s |
| Lanceur, essai à blanc (`performance test`, `rapide sans-ombres test`, groupe inconnu) | réglages attendus affichés ; le groupe inconnu est refusé | |
| Banc, `-Resume` sur un fichier de 41 colonnes et un de 45 | résumé et verdict affichés pour les deux | |

Tests ajoutés : `pipeline::tests::draws_are_left_out_by_their_own_list_only` (un pipeline de
dessin fait avec un shader de la liste est écarté ; un calcul ne l'est jamais par cette
liste) ; `framelog` : 45 colonnes, en-tête sur une seule ligne.

Les deux tests intermittents (`inplace::…warp_a_title_buffer_in_place…`,
`pipeline::…a_preparation_leaves_what_is_no_shader_alone`) et la violation d'accès, vue une
fois et pas reproduite en 85 passes : BLOCKERS 25.

**Ce que cela ne prouve pas :** aucun de ces tests ne lance le jeu. Le gain du profil
`performance` a été mesuré ensuite par l'utilisateur (4,68 et 4,45 images par seconde).

### Cycle 2 (26/11, 8 h) : aucun changement du runtime

Rien n'est recompilé, donc la suite de tests n'a pas été relancée. Vérifié hors jeu :

| Vérification | Résultat |
|---|---|
| Lanceur à blanc : `performance`, `performance precis relais:2`, les mêmes avec `fenetres sans:volumetrie-collecte`, `rapide relais`, `maximum`, `rapide` | réglages attendus affichés ; `performance` sans `transparence`, `maximum` et `rapide` inchangés |
| Banc, `-Resume` sur trois lancements passés | ligne « attentes » : 12,8 par image et 154 ms (`performance`), 13 et 224 ms (`rapide relais`, version diagnostic), absente sans relais |
| Cibles des dessins que la liste d'ombres écarte, sur trois images capturées la veille | 4 512 dessins, tous vers des cartes d'ombres, aucun vers l'image |

**Ce que cela ne prouve pas :** ni la vitesse ni l'image avec `precis`, `relais:2`,
`fenetres` ou `sans:volumetrie-collecte` ; ni que retirer `transparence` rend le haut du
corps des personnages.

### Cycles 4 et 5 (26/11, 8 h 30 à 9 h 10) : runtime modifié

Mêmes commandes que plus haut, dans `..\victory-scarlettport`.

| Vérification | Résultat | Durée |
|---|---|---|
| Suite complète après `ab1bec0` et `63caa72` | 784 réussis, 0 échec, 13 ignorés | 56 s |
| Suite complète après `b4371fc` et `38da7cf` | 785 réussis, 0 échec, 13 ignorés | 55 s |
| `clippy`, `xtask policy` (243 fichiers) | rien à signaler, les deux fois | |
| Versions normale et diagnostic | compilées | 24 s et 30 s |

Tests ajoutés (5) :

- `graveyard` (4) : un objet libéré attend ses images **et** ses millisecondes ; son temps
  passé, il attend les soumissions lancées entre-temps ; sans attente, la règle d'origine ;
  tout ce qui est libéré part une fois, dans l'ordre.
- `indirect::…warp_window_for_a_mesh_pipeline_run_as_compute_holds_its_arguments` : sur
  l'adaptateur logiciel, une fenêtre de 16 enregistrements mesh sort comme les arguments de
  leur calcul et de leur dessin (5 groupes demandés : 5 ; 519 pour 100 de place : 100 ;
  vides ou jamais remplis : rien), et le budget de la liste est compté comme avant.

En jeu, par mes propres lancements du banc : voir [PERF_JOURNAL.md](PERF_JOURNAL.md),
« Mes lancements ».

## Ce que ces tests ne prouvent pas

- **Le rendu en jeu** : un test unitaire réussi ne prouve pas que PROJECT GAME s'affiche.
- **Les images par seconde en jeu** : aucune mesure n'a été faite après les changements de la
  session performances. Le seul chiffre en jeu (environ 1 image par seconde en mode Series X)
  date d'avant.
- **La mémoire réelle en jeu** : elle ne se mesure qu'en lançant le jeu.
- **Les tests de charge** : aucun test de charge GPU ou mémoire reproductible n'existe encore.
- **Le remplacement des mesh shaders de bout en bout** : aucun test ne passe un vrai programme
  de la console par les liaisons et les tranches de `draw.rs`. La comparaison avec le vrai
  chemin mesh sur WARP n'est pas faite (elle demande `VICTORY_CONSOLE_DXC`).

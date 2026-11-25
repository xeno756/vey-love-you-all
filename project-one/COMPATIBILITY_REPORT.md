# Rapport de compatibilité

## Licences

| Source | Licence | Dans PROJECT ONE |
|---|---|---|
| Fork `victory-gta6` / VIctory | GPL-3.0-or-later | base du projet |
| XCL (`xcl-dev/xcl`) | GPL-2.0-only | **aucun code**. Le fork contenait `victory-loader/src/console_insn.rs` et `console_insn_decode.rs`, « Ported from XCL … GPL-2.0-only » (décodeur annoncé « unchanged ») : retirés dans le commit « Loader: drop the console-instruction emulation ported from XCL ». |
| Leonida | GPL-3.0-or-later | aucun code ; référence de conception seulement |
| addrlib (AMD) | MIT | déjà dans le fork (`victory-formats/src/texture.rs`, notice conservée) |
| FFmpeg (décodage XMA) | LGPL-2.1+ | déjà dans le fork (`xma.rs`), compatible |
| `bcpack-tables.bin` | hors licence du dépôt | exception documentée par VIctory (`docs/asset-policy.md`) ; origine non précisée par le fork |

GPL-2.0-only et GPL-3.0 sont incompatibles : une œuvre qui mélange les deux ne peut pas être
distribuée. Le retrait du code de XCL ne change rien sur ce processeur : le gestionnaire
d'exceptions d'origine de VIctory (`emulate_amd_only`) couvre `clzero`, `monitorx`/`mwaitx` et
`clwb`, et le Ryzen 7 5700 exécute ces instructions nativement.

## Dépendances (Cargo.lock)

- 100 paquets, dont 90 de crates.io et le reste du dépôt lui-même ; aucune source git.
- Licences : MIT, Apache-2.0, Unlicense, Zlib, 0BSD, CC0, BSD-3 (opus-pure), Unicode-3.0, toutes
  compatibles GPL-3.0.
- Six paquets ne sont tirés que par une dépendance de test ou une cible non Windows (bitflags,
  errno, linux-raw-sys, r-efi, rustix, valuable) ; leurs licences n'ont pas été vérifiées une à
  une.
- Aucune dépendance ajoutée par PROJECT ONE.

## Fonctions de la carte (RX 5600 XT) face aux besoins de PROJECT GAME

| Besoin | Carte | Conséquence | Traitement |
|---|---|---|---|
| Mesh shaders (toute la géométrie du monde) | **non** | géométrie du monde jamais dessinée | remplacement à faire (`docs/technical/04-shaders-ecran-noir.md`) |
| Shader model 6.6 | oui | — | — |
| Vagues de 64 | oui, natives | les programmes wave64 tournent tels quels | — |
| R9G9B9E5 en render target + UAV (mode Series S) | **non** (`E_INVALIDARG`) | texture sans copie, passes qui l'écrivent vides | **corrigé** : R11G11B10_FLOAT, même taille mémoire |
| Ray tracing matériel (DXR) | non | ombres en ray tracing absentes | déjà désactivé par défaut dans le fork |
| Préemption du calcul fine | non (par liste) | un lot de calcul > 2 s provoque une réinitialisation | rapport amélioré ; découpage à faire |

## Environnement de test

- Le validateur DXIL du SDK Windows 10.0.22621 (1.6) est trop ancien pour deux tests
  `emit::rect` : utiliser `VICTORY_DXIL` vers le `dxil.dll` de DXC 1.9.2609. Le moteur
  n'utilise pas ce validateur en jeu (il signe lui-même le DXIL).
- Le test `a_mesh_pipeline_loads_from_the_library`, qui échouait sur une carte sans mesh
  shaders, est corrigé dans le fork lui-même (commit `3930989`, repris par le rebasage).

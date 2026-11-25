# Agent 3 — Formats de la carte et réinitialisations

Le 24/11/2026, dans une copie séparée du fork (base `1684989`), puis intégré dans le commit
« GPU: R9G9B9E5 targets get an R11G11B10 stand-in; removal reports say how long the card had
the work ». Les numéros de ligne sont ceux de la copie de travail de l'agent.

## A. R9G9B9E5 (DXGI 67) en render target + UAV

### Mesuré sur la RX 5600 XT

- `CreateCommittedResource` et `CreateReservedResource` renvoient tous les deux `0x80070057`.
- `CheckFeatureSupport` donne Support1 = `0x8013f0` : ni le bit render target, ni le bit UAV
  typé.

### Avant la correction

- `host_texture` (`resource.rs:1627`) n'applique `target_format` qu'aux textures render target
  ou UAV (`:1655`) et à leurs valeurs d'effacement (`:1675`).
- La table de substitution ne connaissait que les formats 86 et 115.
- La texture était donc refusée et restait sans copie : toutes ses vues étaient nulles, et
  chaque passe qui l'écrit ne dessinait rien. Pas de crash, contrairement à XCL.

### Correction

- Ajout de `(67, 26)` à la table (`resource.rs:260`), seulement si la carte refuse le format,
  et seulement pour les textures render target ou UAV.
- Toutes les vues, les formats de pipeline et les copies passent déjà par cette table.
- Les relectures d'une texture remplacée sont sautées (`list.rs:1071`), comme les copies.

### Mémoire, en 1920x1080

| Format | Octets |
|---|---|
| R9G9B9E5 | 8 847 360 |
| R11G11B10 | 8 847 360 |
| R16G16B16A16 | 16 711 680 |

### Tests (lancés sur la carte, aucun ignoré)

- la sonde du pilote et les tailles ;
- la texture créée comme le jeu la crée, ses vues, son format de pipeline, un effacement
  relu sur la carte ;
- les vues UAV et de lecture du format 67.

Avec l'entrée 67 retirée, les trois échouent.

## B. Risque de réinitialisation sans mesh shaders

### Ce qui reste envoyé à la carte

- Les pipelines à mesh shaders sont refusés (`pipeline/host.rs:359`) : leurs dessins directs
  et indirects ne coûtent rien à la carte.
- Tout le calcul reste envoyé, y compris celui qui prépare ces dessins : rien ne le relie aux
  dessins sautés.

### Ce qui borne le temps de calcul

- **Limites de boucles par voie** : 128 (pixel), 256 (sommets et mesh), 4 096 (calcul)
  (`lower.rs:10542`).
- **Budget indirect** : 2^27 fils par liste, 2^20 groupes par lancement (`indirect.rs:49-51`).
- **Rien ne borne une soumission entière** : un seul `ExecuteCommandLists`, puis une attente
  sans limite (`host.rs:4056, 4065`).

### Préemption mesurée

- Le graphisme est préempté entre deux primitives.
- Le calcul ne l'est qu'entre deux listes de commandes.

Un lot de calcul de plus de 2 s provoque donc une réinitialisation, ce qui correspond aux
lots de 2,5 à 3 s observés.

### Fait

Le rapport de perte de la carte, dans le journal et dans `crashes.log`, indique désormais :
- combien de millisecondes la carte avait le travail ;
- si cette durée atteint les 2 s du délai de Windows ;
- le nom de la carte et ses niveaux de préemption.

### Conçu, pas écrit : découper les longs travaux de calcul

1. Réutiliser le découpage de listes existant (`list.rs:299`) au-delà d'une estimation de
   travail.
2. Envoyer chaque morceau séparément, avec une seule attente à la fin.
3. N'activer ce découpage que si la carte préempte le calcul par liste.

**Obstacle** : l'état des ressources revient à « commun » entre deux `ExecuteCommandLists`.
**Limite** : ça n'aide pas un lancement de calcul seul de plus de 2 s.

## Tests (copie de l'agent)

- `cargo test -p victory-gpu --locked` : 113 réussis, 1 échec (le test des mesh shaders, déjà
  en échec avant), 1 ignoré.
- Clippy `-D warnings` sans erreur.

# Agent 2 — Mémoire et processeur

Le 24/11/2026, dans une copie séparée du fork (base `1684989`), puis intégré dans le commit
« GPU: upload memory for lists waits for room instead of failing ». Les numéros de ligne sont
ceux de la copie de travail de l'agent.

## Cause des 620 refus de 4 Mio

- **Taille** : 4 Mio est la taille d'un bloc de travail de liste (`SCRATCH_CHUNK`,
  `host.rs:1340`). Les listes en prennent pendant l'enregistrement, surtout pour copier des
  tampons de constantes à chaque dessin (`draw.rs:395, 2009, 2190`).
- **Réserve** : les blocs rendus au reset d'une liste sont gardés jusqu'à 128 Mio
  (`SPARE_MOST`, `host.rs:1346`). Mais rien ne limitait les blocs **détenus**, ni les tampons
  de transfert par copie (`HostList::upload`) ni ceux de plus de 4 Mio.
- **Mémoire concernée** : tous ces tampons sont pris sur la mémoire du PC partagée avec la
  carte, comme les copies de la mémoire du jeu et le débordement de la carte. Windows en donne
  environ la moitié de la RAM : ~8 Go ici, 16 Go chez l'auteur.
- **Suite probable** : quand un bloc pour une vue racine est refusé, le dessin part sans
  cette vue (`draw.rs:1003`), une cause plausible de la perte de la carte qui a suivi
  (`0x887A0006`).
- **Version concernée** : le journal d'origine date d'avant la réserve de blocs du fork, qui
  gardait alors chaque bloc pendant toute la vie de la liste.

## Fils de travail et allocations qui grossissent

| Élément | Valeur ici | Où |
|---|---|---|
| Fils de pipelines | la moitié des fils logiques, 2 à 8 → 8 | `pipeline/make.rs:296` |
| Fils de copie larges | un quart, 2 à 6 → 4, seulement avec `VICTORY_WIDE_BACK` / `VICTORY_WIDE_MERGE` | `shadow.rs:975` |
| DirectStorage | 1 par fil logique → 16, tampon de 32 Mio | `victory-hle/.../dstorage.rs:423, 334` |
| Traduction hors jeu (`translate`) | tous les fils | `apps/victory/src/translate.rs:183` |
| Audio | 1 fil | `audio.rs:660` |
| Tampon de transfert par texture | libéré par le balayage d'inactivité | `texture.rs:1028` |
| Copies de tampons sur la carte | 16 à 512 Mio, si 20 % du budget reste libre | `shadow.rs:148, 212, 267` |
| Instantanés | 512 Mio ; les instantanés conservés sont facultatifs | `shadow.rs:836` |
| Listes inactives | 128 listes / 256 Mio | `list.rs:225` |
| Tas de descripteurs | 1 000 000 vues + 2^18 emplacements dynamiques | `descriptor/host.rs:510, 554` |
| Bibliothèque de pipelines | fichier mappé, réduit après 10 s d'inactivité | `pipeline/library.rs:130` |

Lecture du budget : `Host::video_memory` interroge `QueryVideoMemoryInfo` pour la carte et
pour la mémoire partagée. La règle des 90 % ne regardait que la carte : le budget partagé
était seulement écrit dans le journal.

## Changements

Voir `OPTIMIZATION_REPORT.md`, points 1, 2 et 4. Le nombre de fils de travail n'a pas changé :
ses formules ont seulement été déplacées dans des fonctions, pour la ligne `machine:` du
démarrage.

## Tests (copie de l'agent)

- `cargo test -p victory-gpu` : 114 réussis, 1 échec (le test des mesh shaders, déjà en échec
  avant sur cette carte, corrigé depuis dans le fork), 1 ignoré, identique sur trois passages.
- 5 nouveaux tests.
- Clippy `-D warnings` sans erreur, avec et sans `--all-features`.

## Non vérifié

- Le comportement en jeu.
- Les vrais budgets de cette machine : la ligne `machine:` les donnera.
- Les autres utilisateurs de la mémoire partagée, seulement bornés par leurs balayages :
  `shared_buffer` et le transfert des textures.

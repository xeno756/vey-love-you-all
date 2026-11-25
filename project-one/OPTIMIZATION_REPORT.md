# Rapport d'optimisation

Priorité : stabilité sur cette machine, pas les fps. Rien n'a encore été mesuré en jeu :
PROJECT GAME n'a pas été lancé, comme demandé. Chaque point dit ce qui est prouvé et ce qui
ne l'est pas.

## Fait

### 1. Mémoire de transfert des listes de commandes (RAM partagée avec la carte)

- **Problème mesuré** : sur cette machine, une version antérieure a vu 620 refus de mémoire
  de transfert de 4 Mio (`E_OUTOFMEMORY`) en six secondes, puis la perte de la carte. 4 Mio est
  exactement la taille d'un bloc de travail de liste (`SCRATCH_CHUNK`). Rien ne limitait le
  nombre de blocs détenus à la fois, et cette mémoire est prise sur la RAM que Windows partage
  avec la carte (environ la moitié de la RAM, soit ~8 Go ici).
- **Changement** (`victory-gpu/src/host.rs`, `list_upload`, `upload_room`) :
  - sous 256 Mio, les blocs sont toujours créés ;
  - au-delà, seulement tant que la mémoire partagée reste sous 90 % de son budget ;
  - sinon, ou si Windows refuse, les blocs en réserve sont libérés, et la liste attend
    qu'une autre en rende, 2 s au maximum ;
  - après un dépassement, plus aucune attente jusqu'à ce que de la place revienne.
- **Prouvé** : tests `upload_memory_is_made_within_nine_tenths_of_the_shared_budget`,
  `a_wait_lasts_at_most_the_wait_and_none_while_starved`, `upload_memory_for_a_list_is_kept_or_made`
  (sur la carte), `spare::clearing_takes_everything_kept`.
- **Non prouvé** : l'effet en jeu (fin rapide des attentes, ou blocage de 2 s), le nombre réel
  de blocs détenus par PROJECT GAME.

### 2. Plancher de mémoire vidéo pour les textures

- **Avant** : 2 Gio de copies de textures toujours permis, quel que soit le budget.
- **Après** : 2 Gio ou un quart du budget de la carte, le plus petit des deux, soit environ
  1,4 Gio ici et 2 Gio sur une carte de 12 Go (`resource::texture_floor`).
- **Prouvé** : test `the_texture_floor_is_a_quarter_of_the_card_at_most_2_gib`.

### 3. R9G9B9E5 → R11G11B10 (mode Series S)

- **Effet mémoire mesuré** sur la carte, en 1920x1080 :
  - R9G9B9E5 : 8 847 360 octets ;
  - R11G11B10 : 8 847 360 octets (même taille) ;
  - R16G16B16A16, l'autre choix possible : 16 711 680 octets (presque le double), donc écarté.
- **Prouvé** : la texture est créée et effacée sur la carte, et ses vues et son format de
  pipeline suivent.
- **Non prouvé** : le rendu à l'écran (R11G11B10 garde moins de précision : mantisses de 6 et
  5 bits au lieu de 9).

### 4. Profil machine au démarrage

Une ligne `machine:` du journal donne :
- les processeurs et la RAM ;
- les budgets de la carte et de la mémoire partagée ;
- les limites choisies (fils de compilation des pipelines, fils de copie, plancher des
  textures, mémoire de transfert).

C'est la base pour mesurer au prochain lancement.

## Audité, laissé tel quel (déjà adapté à la machine)

| Élément | Valeur ici | Pourquoi ne pas y toucher |
|---|---|---|
| Fils de pipelines | la moitié des fils, 2 à 8 → 8 | proportionnel |
| Fils de copie larges | un quart, 2 à 6 → 4, seulement avec `VICTORY_WIDE_*` | désactivés par défaut |
| DirectStorage | 16 fils, tampon de 32 Mio | décompression ; mémoire fixe et faible |
| Copies de tampons sur la carte | 16 à 512 Mio, seulement si 20 % du budget reste libre | déjà bornées par le budget |
| Copies de textures | sur la carte seulement à la première utilisation (`VICTORY_LAZY_TEXTURES`) | jusqu'ici, celles des objets à mesh shaders ne prenaient pas de place, car ces objets n'étaient jamais dessinés. Avec le remplacement, ils le seront : il faut s'attendre à plus de mémoire vidéo utilisée |
| Traduction des shaders | DXIL écrit directement, sans DXC | le problème de mémoire de DXC (XCL) n'existe pas ici |

## Pistes suivantes, chiffrées par les agents

| Piste | Gain attendu ici | Effort |
|---|---|---|
| Remplacement des mesh shaders | le monde apparaît | dessins directs écrits ; restent les dessins indirects et le regroupement |
| Découpage des lots de calcul de plus de ~1 s en plusieurs listes | moins de réinitialisations (le calcul n'est préempté qu'entre deux listes sur cette carte) | moyen ; l'état des ressources entre listes est à traiter |
| Limite de boucles adaptée (2^20 aujourd'hui en version normale) | moins de passes qui dépassent 2 s | 1 à 2 jours |
| Render targets qui partagent les pages de mémoire vidéo, avec plafond | moins de mémoire vidéo | 1 à 2 semaines |
| Création de pipelines limitée selon la mémoire engagée libre | moins de pics de RAM | 2 à 4 jours |

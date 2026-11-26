# Limiteurs d'images, lancer de rayons, personnages (26/11/2026, 15 h)

Base de mesure : lancement de l'utilisateur à 14 h 01 (`performance`, avec les murs),
`..\victory-logs\20261126-140156-…`. Je ne lance pas le jeu ; tout vient de ce journal, de ses
captures, du code et des tests hors jeu.

## 1. Ce que mesure le lancement de 14 h 01

| Mesure (après 120 s de monde) | Valeur |
|---|---|
| Images par seconde | **4,07** (6,43 à 13 h 14, sans les murs) |
| Médiane / 95 % / 99 % | 242 / 316 / 373 ms |
| Part de la carte dans l'image | **10 à 12 %** |
| Préparation (fil qui soumet) / recopie / reste (le jeu et ses attentes) | ~30 % / ~30 % / ~25 % |
| Mémoire | 16,3 Go engagés, 2,5 à 3,5 Go en RAM, **260 à 300 Mo libres** |
| Descripteurs copiés en octets refaits par image | 58 500 : 9 811 vues de constantes, 5 526 de tampons, 19 712 de mémoire placée, 3 067 de textures, 20 378 « plus une vue » (emplacements vidés) |
| Fin | **plantage du jeu** à 9 min (section 4) |

Les trois postes les plus lourds, par le journal : la passe des vues de tampons choisies par
index sur un fil du jeu (118 à 262 ms par image depuis le miroir des descripteurs), le miroir
lui-même (113 ms par image sur le fil qui soumet), la recopie (~80 ms). La carte travaille
~25 ms par image : **l'image est limitée par le processeur et la mémoire, pas par la carte.**

## 2. Les limiteurs d'images

| Limiteur | Fichier, fonction | Valeur | Joue-t-il ? |
|---|---|---|---|
| Intervalle de trame du jeu (l'API de cadence de la console) | `victory-gpu/src/frame.rs`, `set_interval` / `wait` | le jeu demande **33,333 ms** (30 images/s) ; les menus tournent à 33,3 ms tout juste | **non dans le monde** : une attente en retard finit aussitôt (`VICTORY_LATE_WAITS` non mis) ; à 150-250 ms par image aucune image n'attend |
| Intervalle de présentation | `victory-gpu/src/pacing.rs`, `sync_interval` | autant de rafraîchissements que l'intervalle du jeu en contient, au plus 4 : **4 sur l'écran de 280 Hz** (14,3 ms) | non : une image toutes les 150 ms ne remplit pas la file de présentation |
| Attente de tout ce qui est en vol à la présentation | `victory-gpu/src/queue.rs`, `present` (`settle_for_present`) | sans attente avec le relais (`late_free_present`) ; le journal compte 0 attente « présentation » | non |
| Attentes de la carte par le fil qui soumet | `host.rs`, `submit` / `settle` | 1,5 à 1,8 par image, 19 à 28 ms (13,8 et 120 ms avant `attentes-textures`) | peu |

Il n'y a donc pas de plafond artificiel qui retienne le monde : le plafond de 30 images/s du jeu
ne joue que sous 33 ms par image. Réglages ajoutés (`0b3af3b`), éteints par défaut :
`cadence:N` ou `cadence:libre` (`VICTORY_FRAME_RATE`, l'horloge du jeu), `vsync:N`
(`VICTORY_PRESENT_SYNC`, l'intervalle de présentation). La cadence n'est pas sûre : on ne sait
pas si la simulation du jeu garde son rythme à une autre cadence.

## 3. Le lancer de rayons

- La console construit ses structures (BVH) par son propre programme (`XboxBvhBuilder`) et les
  parcourt dans ses shaders avec une instruction de la carte (`image_bvh_intersect_ray`).
- Chez nous, **le constructeur est éteint** (sans `VICTORY_BVH_BUILDER=1`, ses dispatchs sont
  écartés : « a stage of the ray-tracing structure builder, which is off ») ; les ~1 000
  programmes qui colorent ce qu'un rayon touche (`*:CS_RaytracingResolve`) ne sont pas
  fabriqués ; les ombres et reflets tracés (`rtdirectionalshadows`, `sr_tracing`) sont écartés
  par `performance` ; l'activation des rayons des sondes d'écran est écartée sur AMD.
- L'instruction d'intersection est traduite en code de calcul (`victory-shader/src/emit/intersect.rs`)
  dans les shaders qui la gardent, et parcourt des structures jamais construites.

Le lancer de rayons est donc déjà pratiquement éteint, et la carte ne fait que 10 % de l'image :
des profils RT OFF / LOW / BALANCED ne peuvent rien rendre de mesurable aujourd'hui. Non
mesuré séparément (il faudrait la version `profil` pour le temps par passe).

## 4. Le plantage de 14 h 10

`title+0x4dbd74d : mov r12,[r13+rdi*8]` avec `edi = [r14+0x12300] = 0xffffffff` : le jeu indexe
un tableau par -1 et lit 0x1100224178 (rien). Première fois sur tous les journaux. Seule la garde
des vues multiples (`a5b2dd3`) rend des pages en lecture seule en plus, et les écritures du
noyau dans la mémoire du jeu (la recopie, `NtWriteVirtualMemory` ; les copies de tampons,
`ReadProcessMemory`) échouent sans bruit sur une page en lecture seule : une garde prise sur une
autre vue pouvait refermer un bloc entre son ouverture et l'écriture. Corrigé (`0b3af3b`) :
ces écritures se font avec les gardes tenues ouvertes ; le journal compte celles qui n'ont réussi
qu'ainsi. Cause probable, non démontrée.

## 5. Les personnages

- **Flous** : `performance` retire 3 niveaux de détail à toutes les textures compressées, au
  plus jusqu'à 128 pixels : un visage de 1024 pixels est lu dans une copie de 128.
  `personnages` (dans `performance`) : les textures que lisent les dessins des pixel shaders des
  personnages (`..\victory-profils\details-personnages.txt`, 1 117 shaders) gardent tous leurs
  niveaux sauf un. Non mesuré : plus de mémoire de carte (au plus quelques centaines de Mo
  attendus) et plus de texels à préparer.
- **Visages noirs des PNJ** (le gardien, 14 h 01) : la cause n'est pas trouvée. Écartées :
  les passes `CS_RaytracingResolve` (coloration des rayons, rien à voir), les textures pas encore
  arrivées (4 388 seulement en attente en régime établi). Il faut une capture de la version
  diagnostic avec les cibles de rendu, sur une image où un visage est noir.

## 6. À lire dans le prochain journal

- `copy-backs into title memory … went only once the guards were held open` : si ce nombre
  n'est pas nul, la course de la section 4 avait bien lieu.
- `characters' textures made again with more of their levels` : combien de textures.
- `descriptor records … made again by kind` et les images par seconde.
- `unhandled exception` : le plantage revient-il ?

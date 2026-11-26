# Audit du moteur, 26/11/2026 (phases 1 et 2 de la mission « optimisation profonde »)

Branche de travail : `project-one/scarlettport` (dossier `..\victory-scarlettport`).
Point de restauration : étiquette locale `project-one/avant-moteur-26-11` (`3b6fe0e`).
Machine : Ryzen 7 5700, 16 Go, RX 5600 XT 6 Go (RDNA1, sans mesh shaders matériels).
Je ne lance pas le jeu : tout ce qui suit vient du lancement de l'utilisateur, des
captures qu'il a envoyées, des journaux, des captures enregistrées les jours précédents,
du code, et de tests hors jeu.

## 1. Mesure de départ (lancement de l'utilisateur, 26/11, 11 h 12, `performance`)

Le monde est visible (deux captures : la salle commune et le dortoir de la prison).
Fichiers : `..\victory-logs\20261126-111215-…-performance-qualite-s-relais-fenetres.*`.

| Mesure (après 120 s de monde, 1 102 images) | Valeur |
|---|---|
| Images par seconde (moyenne) | **5,64** |
| Temps d'image médian / 95 % / 99 % / 99,9 % | 175 / 217 / 239 / 287 ms |
| Pire image | 377 ms (6 631 ms au chargement) |
| Préparation (fil qui soumet) | 58 ms médiane |
| Carte (« on the GPU ») | 109 ms médiane |
| Recopie vers le jeu | 27 ms médiane |
| Dessins par image | 2 529 (et 2 630 laissés de côté : les cartes d'ombres) |
| Mémoire de la carte | 3,6 Go sur un budget de 4,3 Go ; 6,0 Go partagés |
| Défauts de page par image | 266 médiane, 7 177 au 95e centile |
| Perte de carte | aucune (fermé par l'utilisateur à 371 s) |

## 2. Où vont les 175 ms d'une image

Lignes du journal, en régime établi :

- **Le fil qui soumet attend la carte 119,6 ms par image**, en 13,8 attentes de tout ce qui
  est en vol. Toutes sont classées « mémoire du jeu lue » ; le détail : 0,8 pour une copie de
  tampon du jeu, 1,2 pour un remplissage de mémoire d'envoi, et les **11,8 autres pour un
  tampon d'envoi de texture qu'une soumission en vol lit encore** (`staging_in_flight`, venu
  de la pile de ScarlettPort). Ce cas attend tout ce qui est en vol, que la texture ait changé
  ou non.
- **La carte reste sans travail 80 ms par image**, dont 60,6 ms pendant que le fil qui soumet
  prépare : préparation et carte s'enchaînent au lieu de se superposer.
- La passe `heap_buffers` (tampons que les shaders choisissent par index) prend 59,5 ms par
  image sur le fil qui enregistre ; les copies de rafraîchissement font 75 Mo par image
  (34,5 ms). Une mémoire sans suivi d'écriture y est recopiée entière à chaque image.
- Le fil qui recopie attend la carte 99,7 ms par image sans rien à poser, et met 29,6 ms à
  poser les vols.
- 742 exécutions indirectes par image (une par opération), 27 483 groupes de calcul mesh.
- « dropped 270 » par image : ce ne sont pas des pipelines ratés. Ce sont les calculs que les
  groupes d'effets écartent exprès, plus les étapes du constructeur de lancer de rayons, qui
  est éteint (journal de la version diagnostic de 10 h 09).

Conclusion : le levier le plus gros côté processeur n'est pas un calcul, c'est une attente
trop large (11,8 par image). Côté carte, le travail (environ 105 ms) dépasse de loin les
100 ms du premier objectif : il faudra aussi la réduire (préréglage du jeu, dessins mesh).

## 3. Diagnostic des deux captures

| Défaut vu | Cause | Partie du moteur | Statut |
|---|---|---|---|
| Rectangles orange sur les murs, intérieur baigné de lumière orange, brouillard éclairé dans les pièces | Aucune carte d'ombres n'est dessinée (`sans-ombres` dans `performance`) : le soleil n'est arrêté par rien, ni pour l'éclairage, ni pour le brouillard volumétrique qui s'en sert. | dessins laissés de côté (`pipeline::leave_out_draws`), groupe `ombres` | **Corrigé en partie** : la cascade la plus proche est dessinée de nouveau (`b3dd536`). Non vérifié en jeu. |
| Visages des PNJ entièrement noirs (silhouettes), corps et bras éclairés | Déjà vu le 25/11 sans aucun réglage de vitesse : pas une régression d'aujourd'hui. Cause trouvée chez notre amont fxceless (`e4245f3`, « the black-face texture fixes ») : des textures envoyées avant que leurs octets ne soient arrivés gardent des zéros. Chez nous, le runtime ne réexamine une texture qu'au **début** d'une lecture de fichier, jamais à sa fin ; et la file DirectStorage, quand elle se soumet seule à mi-remplissage, lance ses lectures vers des pages physiques avant que le jeu n'ait rempli leur liste de pages. | `victory-hle` `dstorage.rs`, `files.rs` ; `victory-gpu` `watch.rs`, `descriptor/host.rs` | **Corrigé, non vérifié** : les deux correctifs amont repris (`84fe251`). |
| Cheveux surexposés (masse jaune et blanche) | Probable : sans cartes d'ombres et sans les ombres du lancer de rayons (`rtdirectionalshadows`, écarté), les cheveux sont éclairés de plein soleil, de partout. | idem | Devrait s'atténuer avec la cascade proche. Non vérifié. |
| Écartés comme cause | Le motif `CS_RaytracingResolve` du groupe `lancer-de-rayons` attrape six passes de personnages (`ped_standard_blendshape:…`, `ped_standard_skin_tattoo_bake:…`), mais aucune ne tourne même quand elles sont permises (0 dans une image `rapide` ; `pipeline/make.rs` ne les fabrique pas sans le constructeur). | — | — |

Outils : `..\victory-logs\outils\ombres.py` (dessins d'ombres d'une capture d'opérations, par
motif et par cible).

## 4. Changements faits (branche fusionnée)

| Commit | Ce qu'il fait | Réglage pour revenir en arrière | Risque |
|---|---|---|---|
| `84fe251` | Les lectures DirectStorage vers des pages physiques attendent le Submit du jeu quand la file s'est soumise seule (fxceless `79cfdd9`) ; les textures sous une lecture terminée vers une adresse ordinaire sont réexaminées (fxceless `e4245f3`). | `VICTORY_DSTORAGE_HOLD=0`, `VICTORY_READ_LANDED=0` | Des lectures retenues un peu plus longtemps (jusqu'au Submit du jeu) ; plus de textures réexaminées au chargement. |
| `3018756` | `VICTORY_STAGING_PRECISE=1` (mot `attentes-textures`, **éteint par défaut**) : le tampon d'envoi d'une texture n'attend que la soumission qui le lit encore, et seulement si la texture a changé. L'autre moitié de `precis` (lectures de la mémoire du jeu, comme `relais:2`) n'est pas prise. | ne pas donner le mot | Une texture fausse (texels) si un lecteur était oublié ; pas une perte de carte par adresse sauvage, la panne de `relais:2 precis`. |
| `b3dd536` | `VICTORY_SHADOW_CASCADES=N` : seules les N cascades proches du soleil sont dessinées. `performance` prend N = 1 ; profils `ultra-bas` (A, aucune ombre), `performance` (B), `equilibre` (C : 2 cascades, `textures:2`). | `ultra-bas`, ou `sans-ombres` après le profil | Environ 370 dessins de plus par image (8 ms de carte estimés, non mesuré). |

Tests : 232 de `victory-gpu`, 45 de `victory-loader`, 116 de `victory-hle`, tous réussis
(nouveaux : la règle des cascades, les lectures retenues). Clippy sans avertissement.

## 5. La piste communautaire `ifeyz/champion4`

`ifeyz/champion4` n'existe pas en public (404). Le compte `iFeyz` a un fork `iFeyz/VIctory`
sans aucun commit à lui (0 en avance, 479 en retard sur Marvisak/VIctory). Je n'ai pas
deviné d'autre adresse. Une recherche de code GitHub sur les noms des variables les trouve
dans `avatarmanz/VIctory-7900XTX` (le fork déjà étudié, doc 11), `Belobug/VIctory-Belobug-AMD`
et `DoubleGhosty/cool_fox`, tous en GPL-3.0 : la configuration vient de cette famille.

| Variable | Où elle existe | Ce qu'elle fait | Chez nous |
|---|---|---|---|
| `VICTORY_RELAY=2` | ici | 2e étage du relais | **Dangereuse** : 10 pertes de carte sur 15 avec `precis` |
| `VICTORY_CARD_REHASH_RUNS=1` | ici | passes de contrôle jointes | c'est `vitesse:3`, **dangereuse** |
| `VICTORY_INDIRECT_WINDOW=1` | ici | bornage par fenêtres | déjà dans `performance` |
| `VICTORY_TEXTURE_NO_WAIT=1` | ici | lecture des textures pendant un remappage | **rien à gagner** : 0,0 ms par image d'attente au lancement de 11 h 12 |
| `VICTORY_JOIN_COPIES=1` | ici | copies de tampons jointes | **rien à gagner** : 0 copie sur 28 joignable par image |
| `VICTORY_WIDE_BACK=1` | ici (ScarlettPort) | instantanés de la recopie pris d'un coup | à mesurer, un par un |
| `VICTORY_SKIP_PIPELINES=none` | ici | ne retire rien | sans effet |
| `VICTORY_D3D12_DRED=light` | ici (valeur `1` chez nous) | rapport de panne | déjà actif |
| `VICTORY_CARD_STEPS`, `VICTORY_INDIRECT_BATCH`, `VICTORY_MAP_THREADS`, `VICTORY_TITLE_PRIORITIES`, `VICTORY_CARD_AWAKE`, `VICTORY_SHOW_AFTER`, `VICTORY_AMBIENT`, `VICTORY_THIN`, `VICTORY_JOIN_PARTS`, `VICTORY_UPLOAD_AHEAD`, `VICTORY_RELAY_COPY` | forks 7900XTX et Belobug (et `JOIN_PARTS` chez fxceless) | passes de la carte sans attendre, bornage par lots, fils de mappage, priorités, horloges de la carte, lumière ambiante estimée, traduction sur 8 fils, soumissions jointes, envoi anticipé | **absentes de notre code** : il faudrait reprendre leur code, après la fusion de l'amont (section 6) |
| `VICTORY_NO_PREV_CULL`, `VICTORY_LOW_ARENA_MB`, `VICTORY_RDNA4_CRASHERS`, `VICTORY_DIMS_NONUNIFORM`, `VICTORY_REFLECT_HOLD` | dans aucun code public trouvé | inconnu | **non vérifiables** : aucune reprise |
| `VICTORY_RT_RESOLVE=0` | ici (ScarlettPort) | les passes `CS_RaytracingResolve` ne sont pas fabriquées | déjà le défaut |

Aucune de ces variables n'est reprise telle quelle.

## 6. Notre amont fxceless a 98 commits d'avance

Depuis notre base (`project-one/upstream-a803c87`, 24/11), `fxceless/victory-fork` a ajouté
98 commits et 39 712 lignes de Rust. Ils annoncent, mesurés sur une RTX 4070 : « the first room
renders right », les visages noirs corrigés, 6,0 → 8,9 images par seconde avec les fenêtres de
dessins, −11,5 ms de préparation (plus de place pour les vols), −8,7 ms (vérifications de
textures sur les fils d'aide), −10 ms de carte (hachage par groupe, l'équivalent de notre
`a191a66`), le chantier des murs manquants (`VICTORY_WALLS_TRACE`). Une fusion à blanc dans la
branche stable donne 35 blocs en conflit dans 11 fichiers (`host.rs`, `draw.rs`,
`resource.rs`…) ; la branche fusionnée avec ScarlettPort en aurait davantage. C'est le chantier
le plus prometteur pour l'image comme pour la vitesse, et le plus long : il demande sa propre
phase, avec le piège déjà connu des numéros réservés (emplacements de racine, codes).

## 7. Dépôts cités par la mission

| Dépôt | Licence | Compatible GPL-3.0 | Étudié | Intérêt pour nous |
|---|---|---|---|---|
| ZigZagBoi1/ScarlettPort | (non détectée ; pile VIctory en GPL-3.0) | oui pour la pile VIctory | oui, fusionné le 25/11 | attentes, relais, fenêtres : déjà chez nous |
| fxceless/victory-fork (notre amont) | GPL-3.0 | oui | partiellement (section 6) | **le plus utile** |
| avatarmanz/VIctory-7900XTX, Belobug/VIctory-Belobug-AMD | GPL-3.0 | oui | doc 11 ; variables ci-dessus | réglages de soumission, envoi anticipé |
| jadeowaowa/scarlett-layer | Apache-2.0 | oui (dans ce sens) | non | couche D3D12 en C++, à lire pour les idées |
| R0Y00/Xbox2Windows | MIT | oui | non | recompilation statique, tests de validation |
| brcly/TuxBox | Apache-2.0 | oui | non | chemins Windows et Linux, caches |
| Freminet/ScarlettX | **aucune licence** | **non** : lecture d'idées seulement | non | HLE, mémoire unifiée |
| microsoft/DirectXShaderCompiler | NCSA (LLVM) | outil externe | déjà utilisé (DXC 1.9, `VICTORY_DXIL`) | options d'optimisation (`VICTORY_SHADER_OPT`) |
| GPUOpen-Tools/radeon_gpu_analyzer | MIT (source) | outil externe | non installé | coût des shaders sur GFX1010 : **téléchargement à ton accord** |
| GPUOpen-Tools/radeon_gpu_profiler | binaire AMD | outil externe | non installé | chronologie de la carte : **téléchargement à ton accord** |
| zeux/meshoptimizer | MIT | oui | non | peu utile tant que les maillages viennent du jeu en meshlets traduits à la volée |

## 8. Suite proposée

1. Ton lancement de la version `performance` (ombres du soleil proches, correctifs des données
   streamées) : les visages, les murs, l'éclairage des pièces.
2. Puis `performance attentes-textures`, deux lancements : si la carte tient et que les
   attentes tombent de 13,8 à 2 ou 3 par image, c'est le gain le plus fort à portée.
3. La fusion de l'amont fxceless, en phase à part, branche de côté, avec ses mesures.
4. Le préréglage SafeMode (`prereglage:bas`) pour réduire le travail de la carte.

## 9. Lancements de l'utilisateur de 13 h 05 et 13 h 14

| Lancement | Images par seconde (après 120 s) | Médiane | 95 % | 99 % | Carte | Perte |
|---|---|---|---|---|---|---|
| 11 h 12 `performance` (sans ombres) | 5,64 | 175 ms | 217 ms | 239 ms | 109 ms | non |
| 13 h 05 `performance` (cascade proche) | 5,25 | 188 ms | 239 ms | 275 ms | 118 ms | non |
| 13 h 14 `performance attentes-textures` | **6,43** | **153 ms** | 204 ms | 277 ms | 66 ms | non |

- La cascade proche coûte 13 ms de médiane ; `attentes-textures` en rend 35 : `performance`
  le prend depuis 14 h (`bb988d4`, un seul lancement mesuré ; `VICTORY_STAGING_PRECISE=0` le
  retire).
- Capture de 13 h 14 : **le visage de l'héroïne est texturé** (il ne l'était pas le matin) ;
  celui d'une PNJ assise reste taché de sombre (texture partielle ? 4 061 en attente). **Les
  murs du dortoir manquent** : on voit le décor extérieur, la brume orange et le soleil sur
  la tenue viennent de là. Le défaut des murs est connu depuis le 25/11 (BLOCKERS 21 : 3
  captures sur 7, avant tout changement d'aujourd'hui). La retenue des lectures DirectStorage
  n'y est pour rien dans ce lancement : la file ne s'est soumise seule qu'une fois, sans rien
  retenir (à 13 h 05 en revanche, 2 079 lectures retenues d'un coup, durée inconnue : le
  journal la dit maintenant).
- Notre amont écrit que la première pièce s'affiche juste (murs, textures, personnes, lampes)
  depuis deux correctifs, tous deux absents de notre arbre : le miroir des enregistrements de
  descripteurs que le jeu copie en octets (40 000 par image chez lui, leurs emplacements
  vides ou périmés) et la garde de toutes les vues d'une même page (murs 4 fois sur 4 au lieu
  de 2 sur 5). Repris tous deux (`a5b2dd3`), avec un test qui échoue sans le second.
- L'écran du sélecteur de profil (fond bleu nuit, 13 dessins) : même couleur que les écrans
  du début enregistrés le 25/11 en `rapide` ; la vidéo du menu se décode à 30 images par
  seconde dans les trois lancements. Pas de régression visible ; s'il manque un fond, ce
  n'est pas nouveau. Non vérifié.

## 10. Lancement de 13 h 47 : les murs sont revenus, la vitesse a baissé

Captures de l'utilisateur : **le dortoir a ses murs** (plafonniers, porte, sanitaires, sol),
avec les deux correctifs de l'amont (`a5b2dd3`). La cinématique d'ouverture montre encore des
visages noirs et des cheveux très clairs, avec 62 943 textures en attente à ce moment.

Le lancement a été fermé après 125 s de monde, pendant le chargement : pas de mesure en
régime établi. Aux mêmes moments du monde :

| Tranche de monde | 13 h 14 | 13 h 47 |
|---|---|---|
| 0 à 30 s | 3,23 images/s | 2,21 |
| 30 à 60 s | 4,08 | 1,60 |
| 60 à 90 s | 5,17 | 2,50 |
| 90 à 130 s | 6,74 | 3,80 |

Où est passé le temps (journal) :
- les enregistrements que le jeu copie en octets : 403 000 regardés et **58 000 refaits par
  image**, 63 à 110 ms sur le fil qui soumet ;
- la passe des vues de tampons choisies par index (`heap_buffers`), sur un fil du jeu qui
  enregistre : de 2 500 emplacements changés et 19 à 28 ms par image à **41 000 à 143 000
  emplacements et 118 à 262 ms** ; ses vues gardées passent de 84 000 à 284 000 ;
- la garde des vues multiples : environ 1 200 vues rouvertes par image (autant de copies qui
  seraient restées périmées), le temps du noyau de 250 à 400-860 ms par image (tous fils) ;
- deux fois plus de textures en attente au début du monde (62 943 contre 29 139) : le miroir
  en fait connaître davantage, d'où les visages noirs plus longtemps dans la cinématique.

L'amont a eu le même emballement (26 000 à 43 000 enregistrements refaits par image, puis la
passe des tampons) et l'a ramené en laissant les emplacements des tables à leurs dessins : nous
le faisons (179 700 laissés par image), il en reste 58 000. `762c7b8` dit maintenant de quelle
sorte ils sont (vues de constantes, de tampons, de mémoire placée, de textures) : c'est la
donnée qui manque pour les traiter sans perdre les murs.

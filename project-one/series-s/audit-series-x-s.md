# Audit mesuré — Series X contre Series S (27/11/2026)

Exécutable : `game_scarlett_master.exe`, SHA-256 `e95020b2fa4a721db3df59fdecf75621730ac559d8835252f6c3b658d5a08993` (version 1.0.2.1).
Runtime : scarlettport. Parties du banc (`banc.ps1`) : appui sur A jusqu'au monde, cinématique d'ouverture, cellule de la prison, 8 à 12 min de monde.

## Le profil réellement chargé (phase 2)

Vérifié octet par octet dans ce binaire (base préférée 0x140000000) :
- 0x5b56bc0 (sélecteur) : `call 0x501ad20 ; cmp eax, 2 ; mov eax, 0xBEEFBABE ; mov edx, 0xDEADBEEF ; cmove edx, eax ; mov [rsi+0x6f0], edx ; … jmp 0x1d704f0` ;
- 0x501ad20 : `call 0x75140 ; test al, al ; jnz → 1 ; sinon 2` ;
- 0x75140 lit le type de la console (`cmp eax, 6` à 0x7518a : 6 = Lockhart). Le type vient de la valeur `ConsoleMode` que nous donnons : 0x64 pour une Series X (type 7), 0x60 pour une Series S (type 6).

Donc : console Series S → 0x75140 vrai → 0x501ad20 rend 1 → clé **0xDEADBEEF (Lockhart)**.

**Bug trouvé (S-005)** : notre profil `performance` posait le crochet `VICTORY_SERIES_S_QUALITY` (6 remplacé par 7) même en Series S, ce qui faisait choisir Anaconda à une console Series S. Corrigé par `d08552f`. Conséquence pour la comparaison : **notre « Series X performance » tourne déjà avec les choix Lockhart du jeu** (le crochet est fait pour ça) ; la différence entre nos deux lancements est la console déclarée (mémoire 13,5 Go contre 7,6 Go, mode console 0x64 contre 0x60) et ce que le jeu en déduit.

## Tableau

| Système | Series X (10 h 03, `1f0cbd4`) | Series S (11 h 21, `d08552f`) | Action |
|---|---|---|---|
| Images/s | 4,07 | **4,43** | — |
| Temps d'image médian / 95 % / 99 % | 243 / 301 / 345 ms | 217 / 269 / 372 ms | — |
| Préparation (processeur) / carte / recopie | 138 / 99 / 121 ms | 129 / 87 / 108 ms | la recopie reste le plus gros poste : limite de recopie à essayer |
| Dessins par image | 5 229 | 4 547 | — |
| Dessins par taille de cible (5 s) | 1280×720 : 58 214 ; 1280×1280 : 32 960 ; 256×256 : 18 448 | 1280×720 : 49 703 ; 1280×1280 : 39 839 ; 256×256 : 21 698 | 1280×1280 = cartes d'ombre : plus nombreuses en S, à étudier |
| Textures du jeu sans leurs premiers niveaux | 39 578 (69 Go épargnés) | 45 366 (78 Go épargnés) | — |
| Textures en attente à 60 / 120 / 240 s | 24 831 / 21 643 / 25 165 | 26 552 / 21 650 / 25 586 | même débit : le chargement est le nôtre, pas celui du jeu |
| Carte (Mo utilisés / budget) | 3 819 / 4 408 | 3 835 / 4 793 | — |
| Mémoire du PC partagée avec la carte | 6 240 Mo | 6 458 Mo (fenêtres 3 408, copies de tampons 4 116) | **à baisser** (demande de l'utilisateur, 11 h 35) |
| Processus : engagé / en mémoire | 16 357 / 2 940 Mo | 16 637 / 4 668 Mo | — |
| RAM libre min / médiane | 62 / 1 152 Mo | 8 / 1 277 Mo | — |
| Calculs laissés de côté (groupes coupés) | 1,78 M | 1,28 M | reflets en espace écran et lancer de rayons coupés dans les deux |
| Perte de carte / plantage | non / non | non / non | — |

## Mémoire du PC partagée avec la carte (demande de l'utilisateur, 11 h 35)

Ce qui la remplit (colonnes du fichier d'images, partie de 11 h 21) : 821 « fenêtres » de
mémoire du jeu (3,4 Go) et les copies des tampons du jeu, toutes des **copies** dans la mémoire
du PC (la mémoire du jeu existe une seconde fois). Une fenêtre fait 4 Mio, quelle que soit la
part lue.

| Réglage (Series S, médianes après 2 min) | Partagée | Fenêtres | RAM libre | Temps d'image |
|---|---|---|---|---|
| fenêtres de 4 Mio (avant) | 6 426 Mo | 821, 3 392 Mo | 1 317 Mo | 217,0 ms |
| fenêtres de 256 Kio + 1,5 Go de tampons sur la carte | 5 395 Mo | 4 336, 2 411 Mo | 2 042 Mo | 240,6 ms |
| **fenêtres de 1 Mio** + 1,5 Go de tampons sur la carte | **5 781 Mo** | 1 916, 2 825 Mo | 1 523 Mo | 220,7 ms |

Les 1,5 Go de tampons sur la carte ne changent rien (la limite de 1 Go n'était pas atteinte).
Gardé : fenêtres de 1 Mio par défaut en Series S (`963a7ce`) : −650 Mo de mémoire partagée pour
+4 ms par image (dans le bruit de mesure).

Informations manquantes, non mesurées ici : temps de la carte par passe (le profil diagnostic le donne, plus lent), résolution interne exacte de chaque passe, coût du TAAU du jeu.

# Inventaire du « SDK » reçu le 25/11

Dossier fourni par l'utilisateur, hors du dépôt : quatre fichiers d'en-tête C++ produits par un
outil d'extraction tiers pour la version 1.0.2.1 de PROJECT GAME, téléchargés sur un forum.
Rien de ces fichiers n'est copié dans ce dépôt.

## Ce que c'est

| Fichier | Taille | Contenu |
|---|---|---|
| `Basic.hpp` | moins de 1 Ko | 6 types de base (chaîne hachée, vecteurs, tableau) |
| `Classes.hpp` | 6,7 Mo, 229 037 lignes | 44 816 structures avec leurs champs et décalages |
| `Enums.hpp` | 0,9 Mo, 49 348 lignes | 2 563 énumérations |
| `Asserts.hpp` | 11,4 Mo, 136 734 lignes | 98 116 vérifications de taille et de décalage |

Ce sont les **structures des fichiers de données du jeu** (réglages de jeu, personnages,
véhicules, armes, météo…), telles que son moteur les décrit lui-même. 94 % des structures
(41 994) n'ont qu'un nom haché.

## Ce que ce n'est pas

- **Pas un SDK de shaders.** Aucun compilateur, aucune bibliothèque, aucun outil, aucune
  documentation : seulement des descriptions de structures.
- **Aucun des outils AMD** que le plan de travail supposait : ni AGS, ni RGA, ni RGP, ni RMV.
  Aucun n'est installé sur ce PC non plus (vérifié le 25/11).
- **Rien sur le rendu** : 4 structures ont « Shader » dans leur nom, 2 « Graphics », 12
  « Lod », 0 énumération de qualité ou de rendu. Celles qui s'en approchent décrivent des
  données (couleurs prédéfinies, salissures, lumières lointaines), pas le moteur de rendu ni
  ce que la carte exécute.

## Ce qu'on peut en faire

| Besoin | Utile ? |
|---|---|
| Pipelines barycentriques | Non. Déjà résolu sans lui (`12e7c2b` : 0 pipeline refusé). |
| Occupation des shaders, registres, ISA de la carte | Non. Il faut RGA (gratuit, AMD) : à installer si tu le veux. |
| Composition de la mémoire | Non. Nos propres compteurs la donnent (FINDINGS.md). |
| Retrouver un réglage de données du jeu par son nom | Peut-être, comme aide de lecture, si la version correspond à notre copie (non vérifié). |

## Garde-fous

- Licence : aucune. Ces fichiers décrivent un programme propriétaire ; ils ne sont ni
  redistribuables ni compatibles avec la GPL de ce dépôt. Rien n'en est repris.
- Provenance : un forum de triche. Aucun binaire, aucun script : rien à exécuter.
- Équivalents maison pour ce que le plan attendait d'un SDK : temps de carte par commande
  (`profil`), journal par image (`.images.csv`), ligne « memory » de la version diagnostic.

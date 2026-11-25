# Profil matériel (mesuré le 24/11/2026)

| Élément | Valeur | Source |
|---|---|---|
| Processeur | AMD Ryzen 7 5700, 8 cœurs, 16 fils, 3,7 GHz (Zen 3) | WMI `Win32_Processor` |
| RAM | 15,8 Gio (2 × 8 Gio à 3600 MT/s) | WMI `Win32_PhysicalMemory` |
| Limite de mémoire engagée | 46,2 Gio (RAM + fichier d'échange géré par Windows) | WMI `Win32_OperatingSystem` |
| Carte graphique | AMD Radeon RX 5600 XT (RDNA1), 6 128 Mio | registre du pilote |
| Pilote | 32.0.21045.29003 (16/11/2026) | WMI `Win32_VideoController` |
| Direct3D 12 | shader model 6.6, **mesh shaders : non** (tier 0), vagues de 64 natives | sonde D3D12 (`CheckFeatureSupport`) |
| WARP (rendu logiciel) | shader model 6.6, mesh shaders tier 1 | même sonde |
| Préemption | graphique : au niveau des primitives ; calcul : au niveau des listes de commandes | rapport de l'agent 3 (`IDXGIAdapter` desc) |
| R9G9B9E5 en cible + UAV | refusé (`E_INVALIDARG`) | test `the_host_is_asked_for_the_series_s_r9g9b9e5_target` |
| Système | Windows 11 Famille, build 26200 | WMI |
| Délai TDR | réglage Windows par défaut (2 s), non modifié | registre `GraphicsDrivers` |
| Disque | C: 230 Go libres sur 931 | `Get-PSDrive` |
| Outils | Rust 1.98.1 (fixé par le dépôt) et 1.100.0, DXC 1.9.2609, SDK Windows 10.0.22621, Python 3.12.10 | inventaire |

## Ce que ces chiffres impliquent

- **Mémoire partagée avec la carte** : Windows en accorde environ la moitié de la RAM, soit
  près de 8 Go. Copies de la mémoire du jeu, mémoire de transfert et débordement de la carte
  s'y partagent la place. C'est elle qui s'était épuisée (620 refus de 4 Mio).
- **Mémoire du jeu** : jusqu'à 13,5 Gio en mode Series X, 7,6 Gio en mode Series S, à
  prendre sur 15,8 Gio de RAM.
- **Mesh shaders absents** : aucune géométrie du monde ne peut être dessinée sans le
  remplacement décrit dans `docs/technical/04-shaders-ecran-noir.md`.
- **Préemption du calcul par liste** : un lot de calcul de plus de 2 s fait réinitialiser
  la carte par Windows, ce qui correspond aux réinitialisations observées.

Au démarrage, PROJECT ONE écrit maintenant une ligne `machine:` avec ces valeurs et les
limites qu'il en tire (voir `OPTIMIZATION_REPORT.md`).

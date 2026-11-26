# PROJECT ONE: how to use it

*Version française en bas de page.*

PROJECT ONE is a fork of [VIctory](https://github.com/Marvisak/VIctory), by way of
[fxceless/victory-fork](https://github.com/fxceless/victory-fork), with the shader
translator of [ScarlettPort](https://github.com/ZigZagBoi1/ScarlettPort) merged in. It runs
the **Xbox Series X** version of the title on Windows, from **your own** files, and was tuned
on one modest PC: Ryzen 7 5700, 16 GB of RAM, Radeon RX 5600 XT 6 GB.

**This repository contains no game files, code or assets, and nothing in it bypasses any
protection.** You need your own copy of the game. Do not ask for game files here.

## What to expect

Measured on that PC on 2026-11-25 (profile `rapide`, opening scene, 300 s runs):

- The world loads in about a minute, and its textures in about two more.
- About **3 frames per second** (median frame 320 to 330 ms). Not playable yet.
- The card is sometimes lost (`device removed`): 2 runs out of 8 that evening.
- Parts of the picture show blue mosaics, which vary from run to run.

Details: [PERF_JOURNAL.md](PERF_JOURNAL.md), [BLOCKERS.md](BLOCKERS.md),
[KNOWN_ISSUES.md](KNOWN_ISSUES.md) (in French).

## What it needs

- Windows 11 x64, a CPU with AVX2, 16 GB of RAM or more and a large page file.
- A Direct3D 12 GPU with shader model 6.6. **Mesh shaders and barycentrics are not
  needed**: mesh programs run as compute plus a vertex shader, and a generated geometry
  stage supplies barycentrics. Only tested on an AMD RX 5600 XT (driver 32.0.21045.29003).
- Rust (the version pinned in `rust-toolchain.toml`).
- Your own dump of the Xbox Series X version of the title.

## Set up and run

The default branch, `project-one/scarlettport`, is the best version.
`project-one/integration` is the older, stable one (0.3 to 2.5 frames per second).

```powershell
git clone https://github.com/xeno756/vey-love-you-all
cd vey-love-you-all
copy victory.example.toml victory.toml    # set game_dir to your dump, outside the repository
cargo build --release -p victory
project-one\lancer.cmd series-x preparer              # translates the shaders, 3 to 7 min
project-one\lancer.cmd series-x preparer-graphiques   # makes the graphics pipelines, about 1 min
project-one\lancer.cmd series-x 720p textures:2       # run
```

Run `preparer` and `preparer-graphiques` again after any update that changes
`crates/victory-shader`. Each run writes its log and a frame-by-frame CSV in
`..\victory-logs`, beside the clone.

The launcher speaks French. Its main options:

| Option | Effect |
|---|---|
| `series-x` | the Series X version (the only one supported) |
| `720p`, `1080p` | the title renders at that size (`540p` stays on the licence screen) |
| `textures:N` | textures without their first N levels of detail (1 to 6) |
| `rapide` | `720p`, `textures:2`, the title's buffer copies kept on the card, and the effect group `eclairage-indirect` left out (see below) |
| `sans:<group>` | leaves out a group of compute effects (never a draw) |
| `sans-dessins:<group>` | leaves out the DRAWS of a group of passes (`..\victory-profils\dessins-<group>.txt`): it takes geometry away, so keep it for passes the picture can do without |
| `sans-ombres` | `sans-dessins:ombres`: the shadow maps are no longer drawn; the world stays lit, without cast shadows |
| `performance` | everything measured or expected to be faster, picture quality second: `maximum`, plus `sans-ombres`, `relais` and the title's own low settings. Not measured yet |
| `traces` | diagnostics build: on a card loss, the log names the last commands |
| `capture` | diagnostics build: saves frames |
| `test` | prints what would run, runs nothing |

The diagnostics build is made with
`cargo build --profile diag -p victory --features diagnostics`, and needs its own
`preparer` (add `traces` to the command).

### Effect groups

A group is a text file in `..\victory-profils\<group>.txt`, beside the clone. Each line is
part of the name of compute shaders to leave out. `preparer` lists the title's shader names
in `shader-names.txt`, in the cache folder (`%LOCALAPPDATA%\victory\cache-scarlettport`).
These lists come from the title, so they are not in the repository.

`rapide` needs a group named `eclairage-indirect`: the passes of the title's indirect diffuse
lighting and indirect light probes. Without the ray tracing they rely on, they paint the
picture pink and blue. Until you have written that file, run
`series-x 720p textures:2` instead.

`sans-ombres` and `performance` need a group of draws named `ombres`
(`..\victory-profils\dessins-ombres.txt`): parts of the names of the shaders that draw the
world into the shadow maps (the sun's cascades and the local lights). Leave the depth
pre-pass of the picture itself out of that list: without it nothing is drawn.

## Credits and license

- [VIctory](https://github.com/Marvisak/VIctory) by Marvisak and its contributors: the
  loader, the runtime services, the Direct3D 12 back end and the shader recompiler.
- [fxceless](https://github.com/fxceless/victory-fork): the fork this one starts from.
- [ScarlettPort](https://github.com/ZigZagBoi1/ScarlettPort): its shader translator and
  title hooks, merged on the default branch.
- XCL (GPL-2.0-only) was a reference only; none of its code is here.

GPL-3.0-or-later, as every part above. See [LICENSE](../LICENSE).

---

## En français

Version Xbox Series X du jeu, à partir de **vos propres fichiers** : aucun fichier du jeu ici.
Environ 3 images par seconde sur une RX 5600 XT, pas encore jouable. Installation : copier
`victory.example.toml` en `victory.toml` et y mettre le dossier du jeu, compiler avec
`cargo build --release -p victory`, puis lancer `project-one\lancer.cmd series-x preparer`,
`preparer-graphiques`, et enfin `series-x 720p textures:2`. Le reste des notes est en
français : [README.md](README.md), [REPRISE.md](REPRISE.md).

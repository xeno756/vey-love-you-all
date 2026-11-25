@echo off
rem PROJECT ONE - lance PROJECT GAME avec le runtime compile dans ce depot.
rem
rem   lancer.cmd                     Series X (par defaut)
rem   lancer.cmd series-s            Series S
rem   lancer.cmd series-x capture    version diagnostic : enregistre des images
rem                                  de l'ecran et des tampons internes du jeu
rem   lancer.cmd series-x profil     version diagnostic : mesure le temps de la
rem                                  carte commande par commande (plus lent)
rem   lancer.cmd series-x traces     version diagnostic : si la carte est perdue,
rem                                  le journal donne ses dernieres commandes
rem                                  (sans la lenteur de profil)
rem   lancer.cmd series-x preparer   ne lance pas le jeu : traduit ses shaders
rem                                  et compile d'avance ses pipelines de calcul
rem                                  et ceux de ses programmes mesh
rem   lancer.cmd series-x preparer-graphiques
rem                                  comme preparer, plus les pipelines graphiques
rem                                  que le cache de pipelines du jeu liste (long)
rem   lancer.cmd series-x pipelines-du-jeu
rem                                  sert au jeu les pipelines graphiques de son
rem                                  propre cache, compiles par preparer-graphiques
rem                                  (experimental)
rem   lancer.cmd series-x test       montre ce qui serait lance, sans rien lancer
rem   (capture, profil et test peuvent se combiner ; preparer suit la version
rem   choisie : la version diagnostic a son propre cache de shaders)
rem
rem Definition (le jeu lui-meme calcule a la taille demandee) :
rem   lancer.cmd series-x 1080p             image du jeu en 1920x1080, et son rendu
rem                                         3D calcule a cette meme taille
rem   lancer.cmd series-x 720p              idem en 1280x720 (540p : 960x540)
rem   lancer.cmd series-x sortie:1920x1080  taille de l'image du jeu seule ; son
rem                                         rendu 3D en fait les deux tiers
rem   lancer.cmd series-x echelle:0.5       rendu 3D a cette part de l'image du jeu
rem                                         (0.2 a 1) ; sans sortie:, l'image reste
rem                                         en 3840x2160
rem   lancer.cmd series-x sans-synchro      presente les images sans les caler sur
rem                                         l'ecran, comme avant (pour comparer)
rem   lancer.cmd series-x chien-de-garde    laisse le jeu se fermer lui-meme quand
rem                                         il n'avance plus pendant 30 secondes
rem                                         (desarme par defaut)
rem
rem Graphismes reduits (chaque option se teste seule, puis se combine) :
rem   lancer.cmd series-x textures:2        textures du jeu sans leurs 2 premiers
rem                                         niveaux de detail (1 a 6)
rem   lancer.cmd series-x textures:6 plancher:32
rem                                         textures les plus grossieres : jamais
rem                                         sous 32 pixels de cote (128 par defaut)
rem   lancer.cmd series-x sans:vent         laisse de cote un groupe d'effets ;
rem                                         plusieurs : sans:vent+particules
rem   lancer.cmd series-x minimal           textures:2 et les groupes a faible
rem                                         risque (particules, vent, flou-et-halos,
rem                                         reflets)
rem Un groupe est un fichier ..\victory-profils\<groupe>.txt : des morceaux de
rem noms de shaders de calcul, un par ligne. Seuls des calculs sont laisses de
rem cote, jamais un dessin : aucun modele ne disparait par ces listes.
rem
rem Profils (les options ci-dessus deja reglees pour ce PC) :
rem   lancer.cmd series-x rapide            720p et textures:2 : 4 fois moins de
rem                                         pixels a calculer, textures 16 fois plus
rem                                         legeres. Aucun effet n'est laisse de
rem                                         cote : l'image doit rester juste
rem   lancer.cmd series-x maximum           720p, textures:3 et tous les groupes
rem                                         d'effets sauf volumetrie (sans elle
rem                                         l'image peut etre noire) : reflets,
rem                                         ombres et eclairage peuvent etre faux
rem   (540p : le jeu est reste sur l'ecran des licences au lancement du 25/11 a
rem   10 h 13 ; aucun profil ne l'utilise plus)
rem   lancer.cmd series-x rapide vitesse    en plus, moins de copies et de calculs
rem                                         de controle du cote du processeur
rem                                         (reglages de ScarlettPort, non mesures)
rem   lancer.cmd series-x rapide vitesse:2  en plus, ce que les shaders ecrivent
rem                                         n'est recopie que la ou le jeu le relit
rem                                         (plus risque : a essayer en dernier)
rem
rem Le dossier du jeu vient de victory.toml, a la racine du depot (non suivi
rem par git), ou du fichier que VICTORY_CONFIG designe. Chaque lancement garde
rem son journal dans ..\victory-logs.
setlocal
set "ROOT=%~dp0.."
set "CONSOLE=%~1"
if "%CONSOLE%"=="" set "CONSOLE=series-x"
set "CAPTURE="
set "PROFILE="
set "TRACES="
set "PREPARE="
set "DRY="
set "GRAPHICS="
set "TEXTURES="
set "SANS="
set "SORTIE="
set "ECHELLE="
set "WATCHDOG="
set "VITESSE="
set "PROFIL="
:args
if "%~2"=="" goto args_done
if /i "%~2"=="rapide" call :rapide
if /i "%~2"=="maximum" call :maximum
if /i "%~2"=="capture" set "CAPTURE=1"
if /i "%~2"=="profil" set "PROFILE=1"
if /i "%~2"=="traces" set "TRACES=1"
if /i "%~2"=="preparer" set "PREPARE=1"
if /i "%~2"=="preparer-graphiques" set "PREPARE=1"
if /i "%~2"=="preparer-graphiques" set "GRAPHICS=--graphics-pipelines"
if /i "%~2"=="test" set "DRY=1"
if /i "%~2"=="pipelines-du-jeu" set "VICTORY_PRECOMPILED_GRAPHICS=1"
if /i "%~2"=="minimal" call :minimal
if /i "%~2"=="chien-de-garde" set "WATCHDOG=1"
if /i "%~2"=="sans-synchro" set "VICTORY_PRESENT_SYNC=0"
if /i "%~2"=="1080p" call :size 1920x1080
if /i "%~2"=="720p" call :size 1280x720
if /i "%~2"=="540p" call :size 960x540
for /f "tokens=1,* delims=:" %%a in ("%~2") do (
    if /i "%%a"=="textures" set "TEXTURES=%%b"
    if /i "%%a"=="plancher" set "VICTORY_TEXTURE_SMALLEST=%%b"
    if /i "%%a"=="sans" call :without "%%b"
    if /i "%%a"=="sortie" set "SORTIE=%%b"
    if /i "%%a"=="echelle" set "ECHELLE=%%b"
    if /i "%%a"=="vitesse" call :vitesse %%b
)
shift /2
goto args
:args_done

set "DIAG="
if defined CAPTURE set "DIAG=1"
if defined PROFILE set "DIAG=1"
if defined TRACES set "DIAG=1"
set "EXE=%ROOT%\target\release\victory.exe"
set "BUILD=cargo build --release -p victory"
set "MODE=normal"
if defined DIAG (
    set "EXE=%ROOT%\target\diag\victory.exe"
    set "BUILD=cargo build --profile diag -p victory --features diagnostics"
    set "MODE=diagnostic"
)
if not exist "%EXE%" (
    echo Runtime introuvable : %EXE%
    echo Compiler d'abord : %BUILD%
    exit /b 1
)
if not defined VICTORY_CONFIG set "VICTORY_CONFIG=%ROOT%\victory.toml"
if not exist "%VICTORY_CONFIG%" (
    echo Configuration introuvable : %VICTORY_CONFIG%
    echo Copier victory.example.toml en victory.toml et y mettre le dossier du jeu.
    exit /b 1
)

set "GAME="
for /f "tokens=1,* delims=:" %%a in ('""%EXE%" config"') do (
    if "%%a"=="game_dir    " set "GAME=%%b"
)
if not defined GAME (
    echo Le runtime ne donne pas de dossier de jeu : verifier %VICTORY_CONFIG%
    exit /b 1
)
set "GAME=%GAME:~1%"
set "TITLE="
for %%f in ("%GAME%\*.exe") do set "TITLE=%%f"
if not defined TITLE (
    echo Aucun executable dans %GAME%
    exit /b 1
)

rem Hors du depot : journaux et images viennent du jeu.
for %%d in ("%ROOT%\..\victory-logs") do set "LOGS=%%~fd"
set "STAMP=run"
for /f %%t in ('powershell -NoProfile -Command "Get-Date -Format yyyyMMdd-HHmmss"') do set "STAMP=%%t"
rem Les graphismes reduits demandes : dans le nom du journal aussi.
for %%d in ("%ROOT%\..\victory-profils") do set "PROFILS=%%~fd"
rem (Branche de la fusion ScarlettPort : ses journaux portent son nom.)
set "TAG=-scarlettport"
set "BAD="
set "VICTORY_SKIP_SHADERS="
if defined SORTIE (
    set "VICTORY_OUTPUT_SIZE=%SORTIE%"
    set "VICTORY_DISPLAY_MODE=%SORTIE%"
    set "TAG=%TAG%-sortie%SORTIE%"
)
if defined ECHELLE (
    set "VICTORY_RENDER_SCALE=%ECHELLE%"
    set "TAG=%TAG%-echelle%ECHELLE%"
)
if not defined WATCHDOG set "VICTORY_NO_WATCHDOG=1"
rem Le journal dit tout ce que la version stable disait (cette branche n'ecrit sinon que ses avertissements).
if not defined VICTORY_LOG set "VICTORY_LOG=info"
if defined TEXTURES (
    set "VICTORY_TEXTURE_SKIP_MIPS=%TEXTURES%"
    set "TAG=%TAG%-textures%TEXTURES%"
)
if not defined SANS goto groups_done
for %%g in (%SANS:+= %) do call :group %%g
if defined BAD exit /b 1
:groups_done
rem Un profil donne son nom au journal : la liste de ses reglages ferait un
rem chemin plus long que Windows n'en accepte.
if defined PROFIL set "TAG=-scarlettport-%PROFIL%"
if defined VITESSE set "TAG=%TAG%-vitesse%VITESSE%"
set "NAME=%STAMP%-%CONSOLE%-%MODE%%TAG%"
if defined PREPARE set "NAME=%STAMP%-preparation-%MODE%"
set "LOG=%LOGS%\%NAME%.err.log"
set "OUT=%LOGS%\%NAME%.out.log"

echo PROJECT ONE
echo   runtime : %EXE%
if defined PREPARE (
    echo   action  : preparation des shaders, le jeu n'est pas lance
) else (
    echo   jeu     : %TITLE%
    echo   console : %CONSOLE%
)
echo   journal : %LOG%
if defined SORTIE echo   sortie  : image du jeu en %SORTIE%
if defined ECHELLE echo   echelle : rendu 3D a %ECHELLE% de l'image du jeu
if defined TEXTURES echo   textures: sans leurs %TEXTURES% premiers niveaux de detail
if defined SANS echo   sans    : %SANS%
if defined VITESSE echo   vitesse : reglages du processeur, niveau %VITESSE%
if "%SORTIE%"=="960x540" echo   attention : en 960x540 le jeu est reste sur l'ecran des licences (25/11, 10 h 13)
if defined CAPTURE (
    for %%d in ("%ROOT%\..\victory-captures") do set "VICTORY_CAPTURE=%%~fd"
    set "VICTORY_CAPTURE_WORLD=1"
    set "VICTORY_DUMP_TARGETS=1"
)
if defined CAPTURE echo   images  : %VICTORY_CAPTURE%
if defined TRACES (
    set "VICTORY_D3D12_DRED=1"
    echo   traces  : garde les dernieres commandes donnees a la carte
)
if defined PROFILE (
    set "VICTORY_GPU_TIMING=1"
    set "VICTORY_D3D12_DRED=1"
    echo   profil  : temps de la carte par commande
)
if defined DRY exit /b 0
if not exist "%LOGS%" mkdir "%LOGS%"
if defined CAPTURE if not exist "%VICTORY_CAPTURE%" mkdir "%VICTORY_CAPTURE%"

if not defined PREPARE goto title
"%EXE%" prepare --compute-pipelines --mesh-pipelines %GRAPHICS% 2>"%LOG%"
echo Preparation terminee, code %ERRORLEVEL%. Journal : %LOG%
exit /b %ERRORLEVEL%
:title
rem Si la carte est perdue, le journal donne l'adresse fautive et ce qui s'y
rem trouvait : la panne du 25/11 ne vient qu'une fois sur deux et jamais dans la
rem version diagnostic. (VICTORY_D3D12_DRED vide avant le lancement : sans.)
if not defined VICTORY_D3D12_DRED set "VICTORY_D3D12_DRED=1"
rem Une ligne par image, a cote du journal.
set "VICTORY_FRAME_CSV=%LOGS%\%NAME%.images.csv"
"%EXE%" run "%TITLE%" --execute --console %CONSOLE% 1>"%OUT%" 2>"%LOG%"
echo Termine, code %ERRORLEVEL%. Journal : %LOG%
rem Le dernier journal, la ou les anciens lancements l'ecrivaient.
copy /y "%LOG%" "%TEMP%\victory-err.log" >nul
copy /y "%OUT%" "%TEMP%\victory-out.log" >nul
endlocal
exit /b 0

rem Une definition reelle : l'image du jeu a cette taille, et son rendu 3D aussi.
:size
set "SORTIE=%~1"
if not defined ECHELLE set "ECHELLE=1"
goto :eof

rem Le profil minimal : les textures sans 2 niveaux, et les groupes a faible risque.
:minimal
if not defined TEXTURES set "TEXTURES=2"
call :without "particules+vent+flou-et-halos+reflets"
goto :eof

rem Le profil rapide : ce que le lancement du 25/11 a montre de plus couteux sur
rem ce PC (la carte 77 %% du temps d'image en 2560x1440, sa memoire depassee de
rem 2 Go, 30 000 textures en attente de place).
:rapide
set "PROFIL=rapide"
call :size 1280x720
if not defined TEXTURES set "TEXTURES=2"
goto :eof

rem Le profil maximum : tout ce qui peut etre reduit sans retirer un dessin.
rem (1280x720 est la plus petite taille vue en jeu : en 960x540 le jeu n'a pas
rem depasse l'ecran des licences.)
:maximum
set "PROFIL=maximum"
call :size 1280x720
if not defined TEXTURES set "TEXTURES=3"
call :without "particules+vent+flou-et-halos+reflets+lancer-de-rayons+decals+transparence+eclairage-indirect"
goto :eof

rem Les reglages de vitesse de ScarlettPort du cote du processeur : 1, les
rem controles de contenu plus legers ; 2, en plus, la recopie de ce que les
rem shaders ecrivent limitee a ce que le jeu relit.
:vitesse
set "VITESSE=%~1"
if not defined VITESSE set "VITESSE=1"
set "VICTORY_FAST_HASH=1"
set "VICTORY_KNOWN_WRITES=1"
if "%VITESSE%"=="2" set "VICTORY_COPY_BACK_ONLY_READBACK=1"
goto :eof

rem Ajoute des groupes d'effets (separes par +) a ceux a laisser de cote.
:without
if defined SANS (set "SANS=%SANS%+%~1") else set "SANS=%~1"
goto :eof

rem Ajoute la liste d'un groupe d'effets a celles que le runtime lit.
:group
if not exist "%PROFILS%\%~1.txt" (
    echo Groupe d'effets inconnu : %~1
    echo   attendu : %PROFILS%\%~1.txt
    set "BAD=1"
    goto :eof
)
if defined VICTORY_SKIP_SHADERS (
    set "VICTORY_SKIP_SHADERS=%VICTORY_SKIP_SHADERS%;%PROFILS%\%~1.txt"
) else (
    set "VICTORY_SKIP_SHADERS=%PROFILS%\%~1.txt"
)
set "TAG=%TAG%-sans-%~1"
goto :eof

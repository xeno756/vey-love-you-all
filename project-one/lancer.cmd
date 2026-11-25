@echo off
rem PROJECT ONE - lance PROJECT GAME avec le runtime compile dans ce depot.
rem
rem   lancer.cmd                     Series X (par defaut)
rem   lancer.cmd series-s            Series S
rem   lancer.cmd series-x capture    version diagnostic : enregistre des images
rem                                  de l'ecran et des tampons internes du jeu
rem   lancer.cmd series-x profil     version diagnostic : mesure le temps de la
rem                                  carte commande par commande (plus lent)
rem   lancer.cmd series-x preparer   ne lance pas le jeu : traduit ses shaders
rem                                  et compile ses pipelines de calcul d'avance
rem   lancer.cmd series-x test       montre ce qui serait lance, sans rien lancer
rem   (capture, profil et test peuvent se combiner ; preparer suit la version
rem   choisie : la version diagnostic a son propre cache de shaders)
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
set "PREPARE="
set "DRY="
:args
if "%~2"=="" goto args_done
if /i "%~2"=="capture" set "CAPTURE=1"
if /i "%~2"=="profil" set "PROFILE=1"
if /i "%~2"=="preparer" set "PREPARE=1"
if /i "%~2"=="test" set "DRY=1"
shift /2
goto args
:args_done

set "DIAG="
if defined CAPTURE set "DIAG=1"
if defined PROFILE set "DIAG=1"
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
set "NAME=%STAMP%-%CONSOLE%-%MODE%"
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
if defined CAPTURE (
    for %%d in ("%ROOT%\..\victory-captures") do set "VICTORY_CAPTURE=%%~fd"
    set "VICTORY_CAPTURE_WORLD=1"
    set "VICTORY_DUMP_TARGETS=1"
)
if defined CAPTURE echo   images  : %VICTORY_CAPTURE%
if defined PROFILE (
    set "VICTORY_GPU_TIMING=1"
    set "VICTORY_D3D12_DRED=1"
    echo   profil  : temps de la carte par commande
)
if defined DRY exit /b 0
if not exist "%LOGS%" mkdir "%LOGS%"
if defined CAPTURE if not exist "%VICTORY_CAPTURE%" mkdir "%VICTORY_CAPTURE%"

if not defined PREPARE goto title
"%EXE%" prepare --compute-pipelines 2>"%LOG%"
echo Preparation terminee, code %ERRORLEVEL%. Journal : %LOG%
exit /b %ERRORLEVEL%
:title
"%EXE%" run "%TITLE%" --execute --console %CONSOLE% 1>"%OUT%" 2>"%LOG%"
echo Termine, code %ERRORLEVEL%. Journal : %LOG%
rem Le dernier journal, la ou les anciens lancements l'ecrivaient.
copy /y "%LOG%" "%TEMP%\victory-err.log" >nul
copy /y "%OUT%" "%TEMP%\victory-out.log" >nul
endlocal

@echo off
rem PROJECT ONE - lance PROJECT GAME avec le runtime compile dans ce depot.
rem
rem   lancer.cmd                 Series X (par defaut)
rem   lancer.cmd series-s        Series S
rem   lancer.cmd series-x test   montre ce qui serait lance, sans rien lancer
rem
rem Le dossier du jeu vient de victory.toml, a la racine du depot (non suivi
rem par git), ou du fichier que VICTORY_CONFIG designe.
setlocal
set "ROOT=%~dp0.."
set "EXE=%ROOT%\target\release\victory.exe"
if not exist "%EXE%" (
    echo Runtime introuvable : %EXE%
    echo Compiler d'abord : cargo build --release -p victory
    exit /b 1
)
if not defined VICTORY_CONFIG set "VICTORY_CONFIG=%ROOT%\victory.toml"
if not exist "%VICTORY_CONFIG%" (
    echo Configuration introuvable : %VICTORY_CONFIG%
    echo Copier victory.example.toml en victory.toml et y mettre le dossier du jeu.
    exit /b 1
)
set "CONSOLE=%~1"
if "%CONSOLE%"=="" set "CONSOLE=series-x"

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

set "LOG=%TEMP%\victory-err.log"
set "OUT=%TEMP%\victory-out.log"
echo PROJECT ONE
echo   runtime : %EXE%
echo   jeu     : %TITLE%
echo   console : %CONSOLE%
echo   journal : %LOG%
if /i "%~2"=="test" exit /b 0

"%EXE%" run "%TITLE%" --execute --console %CONSOLE% 1>"%OUT%" 2>"%LOG%"
echo Termine, code %ERRORLEVEL%. Journal : %LOG%
endlocal

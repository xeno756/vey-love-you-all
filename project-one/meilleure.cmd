@echo off
rem PROJECT ONE - lance la meilleure version : la branche fusionnee avec ScarlettPort
rem (dossier ..\victory-scarlettport, a cote de ce depot), avec les memes options
rem que lancer.cmd.
rem
rem   meilleure.cmd                      Series X
rem   meilleure.cmd series-x 1080p       image et rendu 3D en 1920x1080
rem   meilleure.cmd series-x 720p minimal
rem   meilleure.cmd series-x preparer    prepare ses shaders, sans lancer le jeu
rem   meilleure.cmd series-x test        montre ce qui serait lance, sans rien lancer
setlocal
set "AUTRE=%~dp0..\..\victory-scarlettport\project-one\lancer.cmd"
if not exist "%AUTRE%" (
    echo Branche fusionnee introuvable : %AUTRE%
    echo Voir REPRISE.md pour la recreer.
    exit /b 1
)
call "%AUTRE%" %*
exit /b %ERRORLEVEL%

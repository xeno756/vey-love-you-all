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
rem                                         legeres ; sans l'eclairage indirect, qui
rem                                         tachait l'image de rose et de bleu
rem   lancer.cmd series-x maximum           720p, textures:3 et tous les groupes
rem                                         d'effets sauf volumetrie (sans elle
rem                                         l'image peut etre noire) : reflets,
rem                                         ombres et eclairage peuvent etre faux
rem   lancer.cmd series-x rapide qualite-s  en plus, le jeu prend ses propres
rem                                         reglages bas (niveau de detail des
rem                                         modeles, ombres, effets : ceux de la
rem                                         Series S), experimental
rem   (540p : le jeu est reste sur l'ecran des licences au lancement du 25/11 a
rem   10 h 13 ; aucun profil ne l'utilise plus)
rem   lancer.cmd series-x rapide vitesse    en plus, moins de copies et de calculs
rem                                         de controle du cote du processeur
rem                                         (reglages de ScarlettPort, non mesures)
rem   lancer.cmd series-x rapide vitesse:2  DANGEREUX : ce que les shaders ecrivent
rem                                         n'est recopie que la ou le jeu le relit
rem                                         et dans les petites zones. Bien plus
rem                                         rapide, mais la carte est perdue (6
rem                                         lancements sur 7 le 26/11)
rem   lancer.cmd series-x rapide vitesse:3  DANGEREUX : vitesse:2, plus les passes de
rem                                         controle du runtime sur la carte jointes
rem   lancer.cmd series-x rapide relais     les commandes partent vers la carte sans
rem                                         attendre les precedentes : 54 ms de moins
rem                                         par image le 26/11, image propre
rem   lancer.cmd series-x performance       LA version a essayer : tout ce qui est
rem                                         mesure plus rapide, la qualite d'image
rem                                         passant apres. maximum sans son groupe
rem                                         transparence, les cartes d'ombres non
rem                                         dessinees, qualite-s, relais, 256 listes
rem                                         en vol, fenetres, le brouillard collecte
rem                                         en moins, les textures creees 80 ms par
rem                                         image au plus. 6 images par seconde le
rem                                         26/11, 300 s sans perte deux fois, la
rem                                         ou rapide en donne 3
rem   lancer.cmd series-x performance-sur   le profil du matin seul : 4,5 images
rem                                         par seconde
rem   lancer.cmd series-x sans-ombres       les cartes d'ombres ne sont plus dessinees
rem                                         (sans-dessins:<groupe> pour un groupe de
rem                                         ..\victory-profils\dessins-<groupe>.txt)
rem Reglages d'attente et de bornage de ScarlettPort, a donner APRES le profil :
rem   ... performance precis relais:2       DANGEREUX : une texture encore lue par
rem                                         la carte ne fait plus attendre, et une
rem                                         lecture n'attend que les soumissions qui
rem                                         ecrivent ce qu'elle lit. 8 images par
rem                                         seconde, mais 10 pertes de carte sur 15
rem                                         lancements le 26/11
rem   ... performance fenetres              les dessins indirects qui se suivent sont
rem                                         bornes en une passe, pas un par un
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
set "QUALITE="
set "RELAIS="
set "PRECIS="
set "FENETRES="
set "PROFIL="
set "SANS_DESSINS="
:args
if "%~2"=="" goto args_done
if /i "%~2"=="rapide" call :rapide
if /i "%~2"=="maximum" call :maximum
if /i "%~2"=="performance" call :performance
if /i "%~2"=="performance-sur" call :performance_sure
if /i "%~2"=="sans-ombres" call :without_draws "ombres"
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
if /i "%~2"=="qualite-s" call :quality_s
if /i "%~2"=="precis" call :precise
if /i "%~2"=="fenetres" call :windows
if /i "%~2"=="1080p" call :size 1920x1080
if /i "%~2"=="720p" call :size 1280x720
if /i "%~2"=="540p" call :size 960x540
for /f "tokens=1,* delims=:" %%a in ("%~2") do (
    if /i "%%a"=="textures" set "TEXTURES=%%b"
    if /i "%%a"=="plancher" set "VICTORY_TEXTURE_SMALLEST=%%b"
    if /i "%%a"=="sans" call :without "%%b"
    if /i "%%a"=="sans-dessins" call :without_draws "%%b"
    if /i "%%a"=="sortie" set "SORTIE=%%b"
    if /i "%%a"=="echelle" set "ECHELLE=%%b"
    if /i "%%a"=="vitesse" call :vitesse %%b
    if /i "%%a"=="relais" call :relay %%b
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
rem Les passes dont les DESSINS sont laisses de cote (sans-ombres,
rem sans-dessins:groupe) : un fichier ..\victory-profils\dessins-<groupe>.txt
rem par groupe, de la meme forme que ceux des effets. Contrairement a
rem sans:groupe, cela retire des dessins : a reserver aux passes dont l'image
rem peut se passer (les cartes d'ombres).
if not defined SANS_DESSINS goto draws_done
for %%g in (%SANS_DESSINS:+= %) do call :draw_group %%g
if defined BAD exit /b 1
:draws_done
rem Un profil donne son nom au journal : la liste de ses reglages ferait un
rem chemin plus long que Windows n'en accepte.
if defined PROFIL set "TAG=-scarlettport-%PROFIL%"
if defined VITESSE set "TAG=%TAG%-vitesse%VITESSE%"
if defined QUALITE set "TAG=%TAG%-qualite-%QUALITE%"
if "%RELAIS%"=="1" set "TAG=%TAG%-relais"
if "%RELAIS%"=="2" set "TAG=%TAG%-relais2"
if defined PRECIS set "TAG=%TAG%-precis"
if defined FENETRES set "TAG=%TAG%-fenetres"
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
if defined SANS_DESSINS echo   dessins : laisses de cote pour %SANS_DESSINS% (ces passes ne sont plus dessinees)
if defined VICTORY_AMBIENT_FILL if /i not "%VICTORY_AMBIENT_FILL%"=="off" echo   lumiere : rebondie remplie a %VICTORY_AMBIENT_FILL% (rien ne l'ecrit)
if defined VITESSE echo   vitesse : reglages du processeur, niveau %VITESSE%
if defined QUALITE echo   qualite : reglages bas du jeu (Series S), memoire de la Series X
if "%RELAIS%"=="1" echo   relais  : les soumissions partent vers la carte sans attendre chacune
if "%RELAIS%"=="2" echo   relais  : niveau 2, une lecture n'attend que les soumissions qui ecrivent ce qu'elle lit
if defined PRECIS if "%VICTORY_SYNC_PRECISE%"=="1" echo   attentes: precises, une texture n'attend que la soumission qui la copie encore
if defined FENETRES if "%VICTORY_INDIRECT_WINDOW%"=="1" echo   bornage : les dessins indirects qui se suivent sont bornes par fenetres, en une passe
if defined VICTORY_CARD_BUFFERS_MIN_MB echo   copies  : tampons du jeu de %VICTORY_CARD_BUFFERS_MIN_MB% Mio et plus sur la carte (%VICTORY_CARD_BUFFERS_MB% Mio au plus)
if "%VICTORY_TEXTURE_GATE_S%"=="0" echo   arrivee : textures a l'allure ordinaire, sans rattrapage
if defined VICTORY_TEXTURE_SPAN_MS echo   chargement : au plus %VICTORY_TEXTURE_SPAN_MS% ms par image passes a creer des textures
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
rem L'eclairage indirect du jeu repose sur son lancer de rayons, que le runtime
rem n'execute pas encore : calcule sans lui, il couvre l'image de taches roses et
rem bleues (captures du 25/11, 14 h 31). Laisse de cote, les taches partent, les
rem murs restent, les interieurs sont plus sombres, et la carte gagne 90 ms.
call :without "eclairage-indirect"
call :card_copies
call :steady_textures
goto :eof

rem Les textures du monde arrivent a l'allure ordinaire, sans le rattrapage qui
rem prenait jusqu'a 1,5 s par image pendant la premiere minute. Mesure le 25/11
rem (22 h 30 a 23 h 24, deux lancements par reglage) : 15 a 17 images de plus de
rem 0,8 s au lieu de 26 a 34 ; en echange 28 %% de textures en moins a 45 s de
rem monde, 8 %% a 90 s, autant ensuite. VICTORY_TEXTURE_GATE_S=60 avant le
rem lancement : le rattrapage d'avant.
:steady_textures
if not defined VICTORY_TEXTURE_GATE_S set "VICTORY_TEXTURE_GATE_S=0"
goto :eof

rem Les copies des tampons du jeu de 1 Mio et plus dans la memoire de la carte,
rem et ce que les shaders y ecrivent trouve par la carte (sans cela le
rem processeur compare 380 Mo par image). Mesure le 25/11 : processeur 113 ms
rem -> 80, recopie 55 -> 34, 99 %% des images sous 444 ms au lieu de 839, et
rem les cheveux justes. La carte y passe 35 ms de plus.
:card_copies
if not defined VICTORY_CARD_BUFFERS_MIN_MB set "VICTORY_CARD_BUFFERS_MIN_MB=1"
if not defined VICTORY_CARD_BUFFERS_MB set "VICTORY_CARD_BUFFERS_MB=1024"
goto :eof

rem Le profil maximum : tout ce qui peut etre reduit sans retirer un dessin.
rem (1280x720 est la plus petite taille vue en jeu : en 960x540 le jeu n'a pas
rem depasse l'ecran des licences.)
:maximum
set "PROFIL=maximum"
call :size 1280x720
if not defined TEXTURES set "TEXTURES=3"
call :card_copies
call :steady_textures
call :without "particules+vent+flou-et-halos+reflets+lancer-de-rayons+decals+transparence+eclairage-indirect"
goto :eof

rem Le profil performance : tout ce qui a ete mesure plus rapide, la qualite
rem d'image passant apres. En plus de maximum : les cartes d'ombres ne sont plus
rem dessinees (au moins 1 300 dessins indirects par image, un quart du temps de
rem carte au lancement mesure du 25/11 : le monde est eclaire sans ombres
rem portees), les soumissions partent sans attendre chacune (relais : 312 a
rem 325 ms contre 361 a 380 le 26/11), et le jeu prend ses propres reglages bas
rem (qualite-s : 440 ms contre 506 et 3,6 Go de memoire en moins le 25/11).
rem Mesure le 26/11 (banc propre, 2 lancements) : 4,45 et 4,68 images par
rem seconde, mediane 201 et 217 ms, contre 327 a 337 ms avec rapide.
rem Le groupe transparence n'y est plus depuis ce jour : sans la passe qui
rem compose les matieres transparentes, ce qui passe par elle n'arrive pas a
rem l'image, et un personnage proche de la camera y passe (haut du corps absent
rem sur une capture d'ecran du 26/11 ; cause probable, a confirmer en jeu).
rem performance reunit ce qui a ete mesure plus rapide ET a tenu (une seule
rem version a essayer) : en plus du profil du matin (relais au niveau 1), 256
rem listes en vol, les fenetres de memoire qui n'attendent plus les ecritures
rem des tampons qu'elles recouvrent, la comparaison des copies par la carte en
rem groupes, le bornage par fenetres, la collecte du brouillard laissee de
rem cote, et la creation des textures bornee a 80 ms par image. Mesure le
rem 26/11 a 10 h 31 et 10 h 38 (Discord ouvert) : 6,1 et 6,0 images par
rem seconde, 95 %% des images sous 190 et 194 ms, 300 s sans perte deux fois.
rem PAS le relais au niveau 2 ni les attentes precises (precis) : 10 pertes de
rem carte sur 15 lancements avec eux le 26/11, plus vite (8 images par
rem seconde) mais pas jouable. PAS vitesse:2 ni vitesse:3 non plus : ce que les
rem shaders ecrivent n'est plus recopie vers la memoire du jeu.
rem performance-sur : le profil du matin seul (4,5 images par seconde).
:performance
call :performance_base
call :relay
call :windows
call :without "volumetrie-collecte"
if not defined VICTORY_TEXTURE_SPAN_MS set "VICTORY_TEXTURE_SPAN_MS=80"
if not defined VICTORY_LATE_WAIT_LISTS set "VICTORY_LATE_WAIT_LISTS=256"
if not defined VICTORY_WINDOW_PAST_BUFFER_WRITES set "VICTORY_WINDOW_PAST_BUFFER_WRITES=1"
goto :eof

:performance_sure
call :performance_base
set "PROFIL=performance-sur"
call :relay
goto :eof

:performance_base
set "PROFIL=performance"
call :size 1280x720
if not defined TEXTURES set "TEXTURES=3"
call :card_copies
call :steady_textures
call :without "particules+vent+flou-et-halos+reflets+lancer-de-rayons+decals+eclairage-indirect"
call :without_draws "ombres"
call :quality_s
goto :eof

rem Ajoute des groupes de passes (separes par +) dont les dessins sont laisses de cote.
:without_draws
if defined SANS_DESSINS (set "SANS_DESSINS=%SANS_DESSINS%+%~1") else set "SANS_DESSINS=%~1"
goto :eof

rem Ajoute la liste d'un groupe de dessins a celles que le runtime lit.
:draw_group
if not exist "%PROFILS%\dessins-%~1.txt" (
    echo Groupe de dessins inconnu : %~1
    echo   attendu : %PROFILS%\dessins-%~1.txt
    set "BAD=1"
    goto :eof
)
if defined VICTORY_SKIP_DRAW_SHADERS (
    set "VICTORY_SKIP_DRAW_SHADERS=%VICTORY_SKIP_DRAW_SHADERS%;%PROFILS%\dessins-%~1.txt"
) else (
    set "VICTORY_SKIP_DRAW_SHADERS=%PROFILS%\dessins-%~1.txt"
)
set "TAG=%TAG%-sans-dessins-%~1"
goto :eof

rem (Les reglages ci-dessous ne remplacent pas une valeur deja donnee avant le
rem lancement : VICTORY_INDIRECT_WINDOW=0 avant performance le coupe.)
rem Les reglages de vitesse de ScarlettPort du cote du processeur : 1, les
rem controles de contenu plus legers ; 2, en plus, la recopie de ce que les
rem shaders ecrivent limitee a ce que le jeu relit.
rem 3, en plus : les passes de controle que le runtime fait lui-meme sur la
rem carte apres chaque envoi de memoire du jeu (180 a 190 par image le 26/11)
rem sont jointes par suites de morceaux qui se touchent.
rem Pourquoi le niveau 2 revient le 26/11 : avec precis et relais:2, le fil qui
rem recopie attend la carte 60 a 104 ms par image pour relire 570 morceaux
rem (36 Mo) que les shaders ont ecrits, et le fil qui soumet l'attend, lui. La
rem perte de carte du 25/11 (une copie liberee encore lue) est de la famille
rem que c4cd932 a fermee depuis ; rien ne l'a remontre.
:vitesse
set "VITESSE=%~1"
if not defined VITESSE set "VITESSE=1"
set "VICTORY_FAST_HASH=1"
set "VICTORY_KNOWN_WRITES=1"
if "%VITESSE%"=="3" if not defined VICTORY_CARD_REHASH_RUNS set "VICTORY_CARD_REHASH_RUNS=1"
if "%VITESSE%"=="3" goto vitesse_2
if not "%VITESSE%"=="2" goto :eof
:vitesse_2
if not defined VICTORY_COPY_BACK_ONLY_READBACK set "VICTORY_COPY_BACK_ONLY_READBACK=1"
rem (Sans les petites zones l'image du jeu est noire : il relit d'autres
rem resultats que ceux de ses tampons de relecture.)
set "VICTORY_COPY_BACK_SMALL_KB=1024"
goto :eof

rem Les commandes partent vers la carte sans attendre la fin des precedentes
rem (le relais du fork) : 312 a 325 ms contre 361 a 380 le 26/11, image propre
rem (les taches du 25/11 venaient de la lumiere rebondie jamais ecrite).
rem relais:2, son second niveau : une soumission qui lit de la memoire du jeu
rem n'attend que les soumissions qui l'ecrivent, pas tout ce qui est en vol.
rem Mesure du 26/11 avec performance (niveau 1) : le fil qui soumet attend
rem encore la carte 14 fois par image, 135 ms sur 197. Le niveau 2 n'a jamais
rem ete lance ici ; l'amont le laisse eteint (murs de la premiere piece perdus
rem dans ses essais).
:relay
set "RELAIS=%~1"
if not "%RELAIS%"=="2" set "RELAIS=1"
if defined VICTORY_RELAY set "RELAIS=%VICTORY_RELAY%"
set "VICTORY_RELAY=%RELAIS%"
goto :eof

rem Les attentes precises de ScarlettPort (VICTORY_SYNC_PRECISE), avec le
rem relais : une texture dont le tampon d'envoi est encore lu par la carte ne
rem fait plus attendre tout ce qui est en vol ; seule son ecriture attend, et
rem seulement la soumission qui la copie. Au lancement du 26/11, 11 ou 12 des
rem 14 attentes par image venaient de la. Jamais lance ici : le journal dit
rem "VICTORY_SYNC_PRECISE hazard" si une dependance lui echappe.
:precise
if not defined VICTORY_SYNC_PRECISE set "VICTORY_SYNC_PRECISE=1"
set "PRECIS=1"
goto :eof

rem Le bornage par fenetres de ScarlettPort (VICTORY_INDIRECT_WINDOW) : les
rem dessins indirects qui lisent des enregistrements a la suite sont bornes
rem en une passe au lieu d'une passe de calcul avant chaque dessin. Mesure le
rem 25/11 avec d'autres reglages (265 ms contre 293), image a requalifier :
rem ses taches d'alors venaient peut-etre de la lumiere rebondie.
:windows
if not defined VICTORY_INDIRECT_WINDOW set "VICTORY_INDIRECT_WINDOW=1"
set "FENETRES=1"
goto :eof

rem Les reglages bas du jeu lui-meme (ceux qu'il prend sur une Series S), la
rem console restant une Series X pour tout le reste. Son propre agrandisseur
rem d'image est demande avec : celui de la Series S (FSR) calculerait a deux
rem tiers de la taille demandee.
:quality_s
set "VICTORY_SERIES_S_QUALITY=1"
set "VICTORY_OWN_UPSCALER=1"
set "QUALITE=s"
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
rem Sans l'eclairage indirect, rien n'ecrit les textures de lumiere rebondie
rem (R9G9B9E5 de la taille de l'image) que l'eclairage par tuiles lit : elles
rem gardaient ce que la memoire du jeu contenait, la pate bleue des murs et des
rem visages (captures du 25/11, 18 h 48, 19 h 25, 20 h 26). Remplies d'une
rem lumiere faible et constante, elle ne revient plus (20 h 34, 20 h 37).
rem VICTORY_AMBIENT_FILL=off avant le lancement : sans.
if "%~1"=="eclairage-indirect" if not defined VICTORY_AMBIENT_FILL set "VICTORY_AMBIENT_FILL=0.02"
goto :eof

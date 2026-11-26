# PROJECT ONE - banc d'essai : lance le jeu, appuie sur A jusqu'au monde, laisse
# tourner la cinematique d'ouverture, ferme le jeu par son numero de processus,
# puis resume le fichier d'images du lancement.
#
#   powershell -ExecutionPolicy Bypass -File banc.ps1
#   powershell -ExecutionPolicy Bypass -File banc.ps1 -Options "series-x rapide relais" -Series 3 -Nom relais
#   powershell -ExecutionPolicy Bypass -File banc.ps1 -Resume <fichier .images.csv>
#
# Banc propre (-Propre, ou VICTORY_BENCH_CLEAN=1) : refuse de mesurer si un
# programme autre que le systeme tient plus de 100 Mo de la carte. Il nomme
# ces programmes et ne ferme rien lui-meme.
#
# Une serie (-Series N) prepare d'abord les shaders et les pipelines du jeu
# (-SansPreparer pour s'en passer), puis donne une ligne par lancement.
# Un seul banc a la fois (verrou). A cote de chaque fichier d'images, un
# fichier .env.json dit dans quel etat etait la machine : programmes sur la
# carte, pilote, alimentation, fichier d'echange, lien PCIe, horloge,
# temperature et activite de la carte pendant le lancement (capteurs AMD).
#
# Le resume porte sur les images du monde (300 dessins ou plus) apres sa
# premiere minute : mediane, 95e et 99e centiles, plus longue image ; les
# images de plus de 0,8 s sont comptees sur tout le monde (c'est la mesure
# qui separe le mieux deux reglages).
param(
    [string]$Options = "series-x rapide",
    [int]$Monde = 300,
    [int]$Plafond = 480,
    [string]$Variables = "",
    [string]$Resume = "",
    [int]$Series = 1,
    [string]$Nom = "",
    [switch]$Propre,
    [switch]$SansPreparer
)

$culture = [Globalization.CultureInfo]::InvariantCulture
# Le systeme, cette application et le jeu lui-meme ne comptent pas comme tiers.
$systeme = @('dwm', 'csrss', 'explorer', 'System', 'claude', 'victory', 'WindowsTerminal', 'powershell', 'conhost')

function Get-Number($value) {
    $n = 0.0
    if ($null -ne $value -and [double]::TryParse("$value", [Globalization.NumberStyles]::Float, $culture, [ref]$n)) { return $n }
    return 0.0
}

# Les programmes qui tiennent de la memoire de la carte : nom, Mo.
function Get-CardHolders {
    $samples = @()
    try { $samples = (Get-Counter '\GPU Process Memory(*)\Dedicated Usage' -ErrorAction Stop).CounterSamples } catch { return @() }
    $byName = @{}
    foreach ($s in $samples) {
        if ($s.CookedValue -lt 5MB -or $s.InstanceName -notmatch 'pid_(\d+)') { continue }
        $p = Get-Process -Id ([int]$Matches[1]) -ErrorAction SilentlyContinue
        $name = if ($p) { $p.ProcessName } else { 'inconnu' }
        $byName[$name] = [int]($byName[$name]) + [int]($s.CookedValue / 1MB)
    }
    @($byName.GetEnumerator() | ForEach-Object { [pscustomobject]@{ nom = $_.Key; mo = $_.Value } } | Sort-Object mo -Descending)
}

# Les capteurs de la carte par la bibliotheque d'AMD livree avec le pilote
# (atiadlxx.dll). Indices verifies sur cette carte au repos le 25/11 : 1
# horloge graphique (MHz), 2 horloge memoire, 8 temperature, 27 point chaud,
# 14 ventilateur (tr/min), 19 activite (%), 23 puissance (W), 41 voies PCIe.
$adlCode = @'
using System;
using System.Runtime.InteropServices;
public static class BancAdl {
    public delegate IntPtr AllocCallback(int size);
    [DllImport("atiadlxx.dll", CallingConvention = CallingConvention.Cdecl)]
    static extern int ADL2_Main_Control_Create(AllocCallback callback, int enumConnected, out IntPtr context);
    [DllImport("atiadlxx.dll", CallingConvention = CallingConvention.Cdecl)]
    static extern int ADL2_Adapter_NumberOfAdapters_Get(IntPtr context, out int count);
    [DllImport("atiadlxx.dll", CallingConvention = CallingConvention.Cdecl)]
    static extern int ADL2_New_QueryPMLogData_Get(IntPtr context, int adapter, IntPtr data);
    const int Bytes = 4 + 256 * 8;
    static AllocCallback alloc = delegate(int size) { return Marshal.AllocCoTaskMem(size); };
    static IntPtr context = IntPtr.Zero;
    static IntPtr buffer = IntPtr.Zero;
    static int adapter = -1;
    public static bool Open() {
        try {
            if (ADL2_Main_Control_Create(alloc, 1, out context) != 0) return false;
            int n;
            if (ADL2_Adapter_NumberOfAdapters_Get(context, out n) != 0) return false;
            buffer = Marshal.AllocHGlobal(Bytes);
            for (int i = 0; i < n; i++) {
                int[] s = Read(i);
                if (s != null && s[2] != 0) { adapter = i; return true; }
            }
            return false;
        } catch (Exception) { return false; }
    }
    static int[] Read(int index) {
        for (int k = 0; k < Bytes; k++) Marshal.WriteByte(buffer, k, 0);
        if (ADL2_New_QueryPMLogData_Get(context, index, buffer) != 0) return null;
        int[] v = new int[512];
        Marshal.Copy(new IntPtr(buffer.ToInt64() + 4), v, 0, 512);
        return v;
    }
    public static int[] Sample() { return adapter < 0 ? null : Read(adapter); }
}
'@
$adl = $false
try { Add-Type -TypeDefinition $adlCode -ErrorAction Stop; $adl = [BancAdl]::Open() } catch { $adl = $false }

function Get-CardSensors {
    if (-not $adl) { return $null }
    $s = [BancAdl]::Sample()
    if (-not $s) { return $null }
    $v = { param($i) if ($s[2 * $i] -ne 0) { $s[2 * $i + 1] } else { $null } }
    [pscustomobject]@{ horloge = (& $v 1); memoire = (& $v 2); temperature = (& $v 8); point_chaud = (& $v 27); ventilateur = (& $v 14); activite = (& $v 19); puissance = (& $v 23); voies = (& $v 41); vitesse_bus = (& $v 40) }
}

function Get-Median($values) {
    $sorted = @($values | Where-Object { $null -ne $_ } | Sort-Object)
    if ($sorted.Count -eq 0) { return $null }
    $sorted[[int][Math]::Floor($sorted.Count / 2)]
}

# Ce qui ne change pas pendant un lancement.
function Get-Machine {
    $card = Get-CimInstance Win32_VideoController | Where-Object { $_.Name -like '*Radeon*' -or $_.Name -like '*GeForce*' -or $_.Name -like '*Arc*' } | Select-Object -First 1
    $page = Get-CimInstance Win32_PageFileUsage | Select-Object -First 1
    $os = Get-CimInstance Win32_OperatingSystem
    $link = $null
    try {
        $device = Get-PnpDevice -Class Display -Status OK -ErrorAction Stop | Select-Object -First 1
        $keys = 'DEVPKEY_PciDevice_CurrentLinkSpeed', 'DEVPKEY_PciDevice_CurrentLinkWidth', 'DEVPKEY_PciDevice_MaxLinkSpeed', 'DEVPKEY_PciDevice_MaxLinkWidth'
        $p = Get-PnpDeviceProperty -InstanceId $device.InstanceId -KeyName $keys -ErrorAction Stop
        $link = "generation {0} x{1} (au plus {2} x{3}), vu du peripherique" -f $p[0].Data, $p[1].Data, $p[2].Data, $p[3].Data
    } catch {}
    [pscustomobject]@{
        carte = $card.Name
        pilote = $card.DriverVersion
        pilote_date = $(if ($card.DriverDate) { $card.DriverDate.ToString('yyyy-MM-dd') } else { $null })
        alimentation = ((powercfg /getactivescheme) -replace '^.*\((.*)\).*$', '$1')
        echange_mo = $(if ($page) { $page.AllocatedBaseSize } else { 0 })
        echange_utilise_mo = $(if ($page) { $page.CurrentUsage } else { 0 })
        ram_mo = [int]($os.TotalVisibleMemorySize / 1KB)
        engagement_limite_mo = [int]($os.TotalVirtualMemorySize / 1KB)
        pcie = $link
        capteurs_amd = $adl
    }
}

function Get-Summary([string]$Path) {
    $rows = New-Object System.Collections.Generic.List[object]
    foreach ($r in (Import-Csv $Path)) {
        if (-not ("$($r.t_ms)" -match '^[0-9.]+$')) { continue }
        $rows.Add([pscustomobject]@{
            t = Get-Number $r.t_ms; ms = Get-Number $r.frame_ms; cpu = Get-Number $r.cpu_ms; gpu = Get-Number $r.gpu_ms
            back = Get-Number $r.back_ms; draws = Get-Number $r.draws; alive = Get-Number $r.host_tex_alive; due = Get-Number $r.tex_due
            ram = Get-Number $r.ram_free_mb; commit = Get-Number $r.commit_left_mb; shared = Get-Number $r.shared_mb
            budget = Get-Number $r.card_budget_mb; translated = Get-Number $r.shaders_translated; wait = Get-Number $r.pipe_wait_ms
            windows = Get-Number $r.windows_mb
        })
    }
    $first = $rows | Where-Object { $_.draws -ge 300 } | Select-Object -First 1
    if (-not $first) { return $null }
    $world = @($rows | Where-Object { $_.t -ge $first.t })
    $steady = @($world | Where-Object { $_.t -ge $first.t + 60000 })
    if ($steady.Count -eq 0) { return [pscustomobject]@{ monde_s = [int]($first.t / 1000); images = 0 } }
    $sorted = @($steady | ForEach-Object { $_.ms } | Sort-Object)
    $at = { param($p) [int]$sorted[[Math]::Min($sorted.Count - 1, [int]($sorted.Count * $p))] }
    $avg = { param($name) ($steady | Measure-Object -Property $name -Average).Average }
    $when = { param($s, $name) $row = $world | Where-Object { $_.t -ge $first.t + 1000 * $s } | Select-Object -First 1; if ($row) { [int]$row.$name } else { $null } }
    $dueLow = $world | Where-Object { $_.t -gt $first.t + 5000 -and $_.due -lt 200 } | Select-Object -First 1
    $seconds = ($steady[-1].t - $steady[0].t) / 1000
    $sum = (& $avg 'cpu') + (& $avg 'gpu') + (& $avg 'back')
    [pscustomobject]@{
        monde_s = [int]($first.t / 1000)
        images = $steady.Count
        par_seconde = [Math]::Round($steady.Count / [Math]::Max(1, $seconds), 2)
        mediane = (& $at 0.5); p95 = (& $at 0.95); p99 = (& $at 0.99); max = [int]$sorted[-1]
        longues = @($world | Where-Object { $_.ms -gt 800 }).Count
        plus_de_50 = @($sorted | Where-Object { $_ -gt 50 }).Count
        processeur = [int](& $avg 'cpu'); carte = [int](& $avg 'gpu'); recopie = [int](& $avg 'back')
        # Part du temps d'image que font processeur + carte + recopie mis bout a bout.
        bout_a_bout = [Math]::Round($sum / [Math]::Max(1, (& $avg 'ms')), 2)
        vivantes_60 = (& $when 60 'alive'); vivantes_120 = (& $when 120 'alive'); vivantes_240 = (& $when 240 'alive')
        dues_sous_200_s = $(if ($dueLow) { [int](($dueLow.t - $first.t) / 1000) } else { $null })
        ram_libre_min = [int]($world | Measure-Object -Property ram -Minimum).Minimum
        ram_libre_mediane = [int](Get-Median ($world | ForEach-Object { $_.ram }))
        engagement_restant_min = [int]($world | Measure-Object -Property commit -Minimum).Minimum
        partagee_max = [int]($world | Measure-Object -Property shared -Maximum).Maximum
        fenetres_max = [int]($world | Measure-Object -Property windows -Maximum).Maximum
        budget_carte_min = [int]($world | Measure-Object -Property budget -Minimum).Minimum
        shaders_traduits = [int]($world | Measure-Object -Property translated -Sum).Sum
        attente_pipelines_ms = [int]($world | Measure-Object -Property wait -Sum).Sum
    }
}

function Show-Summary($s) {
    if (-not $s) { Write-Output "Le monde n'a pas ete atteint."; return }
    if ($s.images -eq 0) { Write-Output "Moins d'une minute de monde."; return }
    "monde atteint a {0} s ; {1} images mesurees ({2} par seconde)" -f $s.monde_s, $s.images, $s.par_seconde
    "temps d'image : mediane {0} ms, 95 % sous {1}, 99 % sous {2}, plus longue {3}" -f $s.mediane, $s.p95, $s.p99, $s.max
    "images de plus de 0,8 s : {0} ; de plus de 50 ms : {1}" -f $s.longues, $s.plus_de_50
    "par image : processeur {0} ms + carte {1} + recopie {2} = {3:P0} du temps d'image" -f $s.processeur, $s.carte, $s.recopie, $s.bout_a_bout
    "textures vivantes a 60 / 120 / 240 s : {0} / {1} / {2} ; dues sous 200 a {3} s" -f $s.vivantes_60, $s.vivantes_120, $s.vivantes_240, $s.dues_sous_200_s
    "memoire : RAM libre min {0} Mo (mediane {1}), partagee max {2} Mo (fenetres {3}), budget carte min {4} Mo" -f $s.ram_libre_min, $s.ram_libre_mediane, $s.partagee_max, $s.fenetres_max, $s.budget_carte_min
    if ($s.shaders_traduits -gt 20 -or $s.attente_pipelines_ms -gt 500) { "ATTENTION cache froid : {0} shaders traduits en jeu, {1} ms d'attente de pipelines" -f $s.shaders_traduits, $s.attente_pipelines_ms }
}

if ($Resume) { Show-Summary (Get-Summary $Resume); exit 0 }

$launcher = Join-Path $PSScriptRoot "meilleure.cmd"
if (-not (Test-Path $launcher)) { $launcher = Join-Path $PSScriptRoot "lancer.cmd" }
$logs = Join-Path $PSScriptRoot "..\..\victory-logs"

# Un seul banc a la fois, et jamais par-dessus une partie en cours.
$created = $false
$lock = New-Object System.Threading.Mutex($true, "Local\ProjectOneBanc", [ref]$created)
if (-not $created) { Write-Output "Un autre banc tourne deja : rien n'est lance."; exit 2 }
try {
    if (Get-CimInstance Win32_Process -Filter "Name='victory.exe'" | Where-Object { $_.CommandLine -like '* run *' }) {
        Write-Output "Le jeu tourne deja : rien n'est lance."; exit 2
    }
    $clean = $Propre -or $env:VICTORY_BENCH_CLEAN -eq '1'
    $holders = Get-CardHolders
    $third = @($holders | Where-Object { $systeme -notcontains $_.nom -and $_.mo -gt 100 })
    $others = [int]($holders | Measure-Object -Property mo -Sum).Sum
    "carte tenue par d'autres programmes : {0} Mo ({1})" -f $others, (($holders | Select-Object -First 6 | ForEach-Object { "$($_.nom) $($_.mo)" }) -join ", ")
    if ($clean -and $third.Count -gt 0) {
        "BANC PROPRE REFUSE : {0} tiennent plus de 100 Mo de la carte. Ferme-les, ou mesure sans -Propre une serie a part." -f (($third | ForEach-Object { "$($_.nom) ($($_.mo) Mo)" }) -join ", ")
        exit 3
    }
    foreach ($pair in $Variables.Split(";", [StringSplitOptions]::RemoveEmptyEntries)) {
        $name, $value = $pair.Split("=", 2)
        Set-Item -Path "env:$name" -Value $value
    }
    $machine = Get-Machine
    $prepared = $null
    if (-not $SansPreparer) {
        # La version que la serie lance (normale ou diagnostic), sans le profil.
        $build = @('capture', 'profil', 'traces') | Where-Object { " $Options " -like "* $_ *" } | Select-Object -First 1
        $prepared = @()
        foreach ($step in 'preparer', 'preparer-graphiques') {
            $out = & cmd.exe /c "`"$launcher`" series-x $build $step" 2>&1
            $line = @($out | Where-Object { "$_" -match '^shaders |^precompile-graphics|Preparation terminee' }) -join " | "
            $prepared += "${step}: $line"
            if (-not ("$out" -match 'Preparation terminee, code 0')) { Write-Output "La preparation ($step) a echoue : rien n'est mesure. $line"; exit 4 }
        }
        $text = (($prepared -join " ; ") -replace '\s+', ' ')
        "cache prepare : " + $text.Substring(0, [Math]::Min(420, $text.Length))
    }

    $results = @()
    for ($run = 1; $run -le $Series; $run++) {
        $press = Join-Path $env:TEMP ("victory-banc-touches-{0}-{1}.txt" -f $PID, $run)
        Set-Content -Path $press -Value "" -NoNewline
        $env:VICTORY_PRESS_FILE = $press
        $before = Get-CardHolders
        # La carte au repos juste avant : une activite elevee sans le jeu dit
        # qu'un autre programme s'en sert.
        $rest = @(1..3 | ForEach-Object { Start-Sleep -Milliseconds 700; Get-CardSensors })
        $idle = [pscustomobject]@{ activite = Get-Median ($rest | ForEach-Object { $_.activite }); horloge = Get-Median ($rest | ForEach-Object { $_.horloge }); temperature = Get-Median ($rest | ForEach-Object { $_.temperature }) }
        $began = Get-Date
        [void](Start-Process -FilePath "cmd.exe" -ArgumentList "/c", "`"$launcher`" $Options" -WindowStyle Hidden -PassThru)
        $game = $null
        for ($i = 0; $i -lt 60 -and -not $game; $i++) {
            Start-Sleep -Milliseconds 500
            $game = Get-CimInstance Win32_Process -Filter "Name='victory.exe'" | Where-Object { $_.CommandLine -like '* run *' -and $_.CreationDate -ge $began.AddSeconds(-2) } | Select-Object -First 1
        }
        if (-not $game) { Write-Output "Le jeu n'a pas demarre."; exit 1 }
        $id = $game.ProcessId
        $csv = $null; $worldAt = $null; $lastPress = Get-Date; $ended = "ferme par le banc"; $tick = 0
        $sensors = New-Object System.Collections.Generic.List[object]
        while ($true) {
            Start-Sleep -Milliseconds 1000
            if (-not (Get-Process -Id $id -ErrorAction SilentlyContinue)) { $ended = "ferme tout seul"; break }
            if (-not $csv) {
                $csv = Get-ChildItem (Join-Path $logs "*.images.csv") -ErrorAction SilentlyContinue | Where-Object { $_.CreationTime -ge $began.AddSeconds(-2) } | Sort-Object CreationTime | Select-Object -Last 1
            }
            $draws = 0
            if ($csv) {
                $cells = "$(Get-Content $csv.FullName -Tail 1 -ErrorAction SilentlyContinue)".Split(",")
                if ($cells.Count -gt 7) { $draws = Get-Number $cells[6] }
            }
            $now = Get-Date
            if (-not $worldAt) {
                if ($draws -ge 300) { $worldAt = $now }
                elseif (($now - $lastPress).TotalSeconds -ge 2) { try { Set-Content -Path $press -Value "A" -NoNewline -ErrorAction Stop } catch {}; $lastPress = $now }
            } elseif (($now - $worldAt).TotalSeconds -ge $Monde) { break }
            # Les capteurs de la carte toutes les 5 s, dans le monde seulement.
            $tick++
            if ($worldAt -and $tick % 5 -eq 0) { $s = Get-CardSensors; if ($s) { $sensors.Add($s) } }
            if (($now - $began).TotalSeconds -ge $Plafond) { $ended = "ferme par le banc a sa limite de temps"; break }
        }
        $p = Get-Process -Id $id -ErrorAction SilentlyContinue
        if ($p) { [void]$p.CloseMainWindow(); [void]$p.WaitForExit(20000) }
        Remove-Item $press -ErrorAction SilentlyContinue
        Write-Output ("--- lancement {0} sur {1} ({2}) : {3} ; fichier : {4}" -f $run, $Series, $began.ToString('HH:mm:ss'), $ended, $(if ($csv) { $csv.Name } else { "aucun" }))
        if (-not $csv) { continue }
        $summary = Get-Summary $csv.FullName
        Show-Summary $summary
        $log = $csv.FullName -replace '\.images\.csv$', '.err.log'
        $lost = @(Select-String -Path $log -Pattern "host GPU device removed" -ErrorAction SilentlyContinue).Count
        $refused = @(Select-String -Path $log -Pattern "host pipeline refused|records refused" -ErrorAction SilentlyContinue).Count
        $oom = @(Select-String -Path $log -Pattern "0x8007000E" -ErrorAction SilentlyContinue).Count
        $card = [pscustomobject]@{
            echantillons = $sensors.Count
            horloge_mediane = Get-Median ($sensors | ForEach-Object { $_.horloge })
            horloge_min = ($sensors | ForEach-Object { $_.horloge } | Measure-Object -Minimum).Minimum
            horloge_max = ($sensors | ForEach-Object { $_.horloge } | Measure-Object -Maximum).Maximum
            memoire_mediane = Get-Median ($sensors | ForEach-Object { $_.memoire })
            temperature_max = ($sensors | ForEach-Object { $_.temperature } | Measure-Object -Maximum).Maximum
            point_chaud_max = ($sensors | ForEach-Object { $_.point_chaud } | Measure-Object -Maximum).Maximum
            activite_mediane = Get-Median ($sensors | ForEach-Object { $_.activite })
            puissance_mediane = Get-Median ($sensors | ForEach-Object { $_.puissance })
            voies_pcie = $(if ($sensors.Count) { $sensors[0].voies } else { $null })
            vitesse_bus = $(if ($sensors.Count) { $sensors[0].vitesse_bus } else { $null })
        }
        "pipelines refuses : {0} ; carte perdue : {1} ; manque de memoire : {2}" -f $refused, $(if ($lost) { "oui" } else { "non" }), $(if ($oom) { "oui" } else { "non" })
        if ($card.echantillons) {
            "carte pendant le monde : horloge {0} MHz (de {1} a {2}), memoire {3} MHz, {4} degres au plus (point chaud {5}), activite {6} %, {7} W" -f $card.horloge_mediane, $card.horloge_min, $card.horloge_max, $card.memoire_mediane, $card.temperature_max, $card.point_chaud_max, $card.activite_mediane, $card.puissance_mediane
        }
        $entry = [pscustomobject]@{
            nom = $Nom; options = $Options; variables = $Variables; debut = $began.ToString('yyyy-MM-dd HH:mm:ss'); fin = $ended
            propre = [bool]$clean; tiers_sur_la_carte_mo = $others; carte_tenue_avant = $before; carte_tenue_apres = (Get-CardHolders)
            machine = $machine; preparation = $prepared; carte_au_repos = $idle; carte = $card; resume = $summary
            carte_perdue = [bool]$lost; pipelines_refuses = $refused; manque_de_memoire = [bool]$oom
            cache_froid = [bool]($summary -and ($summary.shaders_traduits -gt 20 -or $summary.attente_pipelines_ms -gt 500))
        }
        $entry | ConvertTo-Json -Depth 5 | Set-Content -Path ($csv.FullName -replace '\.images\.csv$', '.env.json') -Encoding UTF8
        $results += $entry
    }

    if ($Series -gt 1 -and $results.Count -gt 0) {
        ""
        "=== serie '{0}' : {1} lancements, banc propre : {2} ===" -f $Nom, $results.Count, $(if ($clean) { "oui" } else { "non" })
        "heure     mediane  95%   99%   max   >0,8s  vivantes 60/120/240     proc+carte+recopie  RAM min  horloge  degres  perdue"
        foreach ($e in $results) {
            $s = $e.resume
            if (-not $s -or $s.images -eq 0) { "{0}  (monde non mesure)  perdue: {1}" -f $e.debut.Substring(11), $e.carte_perdue; continue }
            "{0}  {1,6}  {2,4}  {3,4}  {4,4}  {5,5}  {6,6}/{7,6}/{8,6}  {9,4}+{10,4}+{11,3}       {12,6}  {13,7}  {14,6}  {15}" -f $e.debut.Substring(11), $s.mediane, $s.p95, $s.p99, $s.max, $s.longues, $s.vivantes_60, $s.vivantes_120, $s.vivantes_240, $s.processeur, $s.carte, $s.recopie, $s.ram_libre_min, $e.carte.horloge_mediane, $e.carte.temperature_max, $(if ($e.carte_perdue) { "OUI" } else { "non" })
        }
        $ok = @($results | Where-Object { $_.resume -and $_.resume.images -gt 0 })
        if ($ok.Count -gt 0) {
            "mediane des lancements : mediane {0} ms ; images > 0,8 s {1} (de {2} a {3}) ; 99 % de {4} a {5} ms" -f (Get-Median ($ok | ForEach-Object { $_.resume.mediane })), (Get-Median ($ok | ForEach-Object { $_.resume.longues })), ($ok | ForEach-Object { $_.resume.longues } | Measure-Object -Minimum).Minimum, ($ok | ForEach-Object { $_.resume.longues } | Measure-Object -Maximum).Maximum, ($ok | ForEach-Object { $_.resume.p99 } | Measure-Object -Minimum).Minimum, ($ok | ForEach-Object { $_.resume.p99 } | Measure-Object -Maximum).Maximum
        }
        if ($results | Where-Object { $_.cache_froid }) { "ATTENTION : au moins un lancement a traduit des shaders ou attendu des pipelines en jeu (cache froid)." }
    }
} finally {
    $lock.ReleaseMutex()
    $lock.Dispose()
}

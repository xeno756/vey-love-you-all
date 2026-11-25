# PROJECT ONE - banc d'essai : lance le jeu, appuie sur A jusqu'au monde, laisse
# tourner la cinematique d'ouverture, ferme le jeu par son numero de processus,
# puis resume le fichier d'images du lancement.
#
#   powershell -ExecutionPolicy Bypass -File banc.ps1
#   powershell -ExecutionPolicy Bypass -File banc.ps1 -Options "series-x rapide relais" -Monde 300
#   powershell -ExecutionPolicy Bypass -File banc.ps1 -Resume <fichier .images.csv>
#
# Le resume porte sur les images du monde (300 dessins ou plus) apres sa
# premiere minute : temps d'image median, 95e et 99e centiles, plus longue
# image, images de plus de 50 ms (sous 20 par seconde) et de plus de 200 ms,
# part du processeur, de la carte et de la recopie.
param(
    [string]$Options = "series-x rapide",
    [int]$Monde = 300,
    [int]$Plafond = 480,
    [string]$Variables = "",
    [string]$Resume = ""
)

function Show-Summary([string]$Path) {
    $culture = [Globalization.CultureInfo]::InvariantCulture
    $rows = Import-Csv $Path | ForEach-Object {
        try {
            [pscustomobject]@{
                t = [double]::Parse($_.t_ms, $culture); ms = [double]::Parse($_.frame_ms, $culture)
                cpu = [double]::Parse($_.cpu_ms, $culture); gpu = [double]::Parse($_.gpu_ms, $culture)
                back = [double]::Parse($_.back_ms, $culture); draws = [double]::Parse($_.draws, $culture)
            }
        } catch {}
    }
    $first = $rows | Where-Object { $_.draws -ge 300 } | Select-Object -First 1
    if (-not $first) { Write-Output "Le monde n'a pas ete atteint."; return }
    $world = @($rows | Where-Object { $_.t -ge $first.t + 60000 })
    if ($world.Count -eq 0) { Write-Output "Moins d'une minute de monde."; return }
    $sorted = @($world | ForEach-Object { $_.ms } | Sort-Object)
    $at = { param($p) $sorted[[Math]::Min($sorted.Count - 1, [int]($sorted.Count * $p))] }
    $avg = { param($name) ($world | Measure-Object -Property $name -Average).Average }
    $seconds = ($world[-1].t - $world[0].t) / 1000
    "monde atteint a {0:N0} s ; {1} images mesurees sur {2:N0} s ({3:N2} par seconde)" -f ($first.t / 1000), $world.Count, $seconds, ($world.Count / [Math]::Max(1, $seconds))
    "temps d'image : mediane {0:N0} ms, 95 % sous {1:N0}, 99 % sous {2:N0}, plus longue {3:N0}" -f (& $at 0.5), (& $at 0.95), (& $at 0.99), $sorted[-1]
    "images de plus de 50 ms : {0} ; de plus de 200 ms : {1}" -f @($sorted | Where-Object { $_ -gt 50 }).Count, @($sorted | Where-Object { $_ -gt 200 }).Count
    "par image : processeur {0:N0} ms + carte {1:N0} + recopie {2:N0}" -f (& $avg 'cpu'), (& $avg 'gpu'), (& $avg 'back')
}

if ($Resume) { Show-Summary $Resume; exit 0 }

$launcher = Join-Path $PSScriptRoot "meilleure.cmd"
if (-not (Test-Path $launcher)) { $launcher = Join-Path $PSScriptRoot "lancer.cmd" }
$logs = Join-Path $PSScriptRoot "..\..\victory-logs"
$press = Join-Path $env:TEMP "victory-banc-touches.txt"
Set-Content -Path $press -Value "" -NoNewline
$env:VICTORY_PRESS_FILE = $press
foreach ($pair in $Variables.Split(";", [StringSplitOptions]::RemoveEmptyEntries)) {
    $name, $value = $pair.Split("=", 2)
    Set-Item -Path "env:$name" -Value $value
}
$began = Get-Date
[void](Start-Process -FilePath "cmd.exe" -ArgumentList "/c", "`"$launcher`" $Options" -WindowStyle Hidden -PassThru)
$game = $null
for ($i = 0; $i -lt 60 -and -not $game; $i++) {
    Start-Sleep -Milliseconds 500
    $game = Get-CimInstance Win32_Process -Filter "Name='victory.exe'" | Where-Object { $_.CommandLine -like '* run *' -and $_.CreationDate -ge $began.AddSeconds(-2) } | Select-Object -First 1
}
if (-not $game) { Write-Output "Le jeu n'a pas demarre."; exit 1 }
$id = $game.ProcessId
$csv = $null; $worldAt = $null; $lastPress = Get-Date; $ended = "ferme par le banc"
while ($true) {
    Start-Sleep -Milliseconds 1000
    if (-not (Get-Process -Id $id -ErrorAction SilentlyContinue)) { $ended = "ferme tout seul"; break }
    if (-not $csv) {
        $csv = Get-ChildItem (Join-Path $logs "*.images.csv") -ErrorAction SilentlyContinue | Where-Object { $_.CreationTime -ge $began.AddSeconds(-2) } | Sort-Object CreationTime | Select-Object -Last 1
    }
    $draws = 0
    if ($csv) {
        $cells = "$(Get-Content $csv.FullName -Tail 1 -ErrorAction SilentlyContinue)".Split(",")
        if ($cells.Count -gt 7) { [void][double]::TryParse($cells[6], [Globalization.NumberStyles]::Float, [Globalization.CultureInfo]::InvariantCulture, [ref]$draws) }
    }
    $now = Get-Date
    if (-not $worldAt) {
        if ($draws -ge 300) { $worldAt = $now }
        elseif (($now - $lastPress).TotalSeconds -ge 2) { Set-Content -Path $press -Value "A" -NoNewline; $lastPress = $now }
    } elseif (($now - $worldAt).TotalSeconds -ge $Monde) { break }
    if (($now - $began).TotalSeconds -ge $Plafond) { $ended = "ferme par le banc a sa limite de temps"; break }
}
$p = Get-Process -Id $id -ErrorAction SilentlyContinue
if ($p) { [void]$p.CloseMainWindow(); [void]$p.WaitForExit(20000) }
Write-Output ("{0} ; fichier : {1}" -f $ended, $(if ($csv) { $csv.Name } else { "aucun" }))
if ($csv) {
    Show-Summary $csv.FullName
    $log = $csv.FullName -replace '\.images\.csv$', '.err.log'
    $lost = @(Select-String -Path $log -Pattern "host GPU device removed" -ErrorAction SilentlyContinue).Count
    $refused = @(Select-String -Path $log -Pattern "host pipeline refused|records refused" -ErrorAction SilentlyContinue).Count
    "pipelines refuses : {0} ; carte perdue : {1}" -f $refused, $(if ($lost) { "oui" } else { "non" })
}

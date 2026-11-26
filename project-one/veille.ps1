# PROJECT ONE - veille des depots suivis : lit ce qui a change depuis la
# derniere passe (commits, demandes de fusion) et ecrit un resume.
#
#   powershell -ExecutionPolicy Bypass -File veille.ps1                une passe
#   powershell -ExecutionPolicy Bypass -File veille.ps1 -Installer     tache planifiee, une passe par heure
#   powershell -ExecutionPolicy Bypass -File veille.ps1 -Desinstaller  retire la tache
#
# Ce que la veille fait : des requetes en lecture a l'API de GitHub, par
# l'outil gh deja connecte (aucun jeton n'est lu ni ecrit ici). Elle garde
# des METADONNEES : depot, reference, auteur, date, titre, licence, adresse.
# Ce qu'elle ne fait pas : elle ne telecharge ni code, ni diff, ni fichier
# d'un depot ; elle ne fusionne rien ; elle n'execute rien de ce qu'elle lit.
# Un depot qui disparait est signale avec sa derniere reference connue.
# Elle ne fait aucune requete quand le jeu tourne (les mesures restent propres).
#
# Dossier de travail, hors du depot : ..\..\victory-watch
#   suivi.txt        les sources : "repo proprietaire/nom", et au plus une ligne
#                    "tracker proprietaire/nom chemin/du/fichier.json"
#   etat.json        ou la veille en est, par depot
#   candidats.json   ce qui touche nos blocages, note par mots-cles
#   AAAA-MM-JJ_HH.md le resume de chaque passe (30 lignes au plus)
#   veille.log       journal, coupe a 256 Ko
param([switch]$Installer, [switch]$Desinstaller, [string]$Dossier = "")

$task = "PROJECT ONE veille"
if (-not $Dossier) { $Dossier = Join-Path $PSScriptRoot "..\..\victory-watch" }
$gh = "C:\Program Files\GitHub CLI\gh.exe"

if ($Desinstaller) {
    Unregister-ScheduledTask -TaskName $task -Confirm:$false -ErrorAction SilentlyContinue
    Write-Output "Tache '$task' retiree."
    exit 0
}
if ($Installer) {
    $action = New-ScheduledTaskAction -Execute "powershell.exe" -Argument "-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$PSCommandPath`""
    $trigger = New-ScheduledTaskTrigger -Once -At (Get-Date).AddMinutes(2) -RepetitionInterval (New-TimeSpan -Hours 1)
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 10) -MultipleInstances IgnoreNew -StartWhenAvailable
    [void](Register-ScheduledTask -TaskName $task -Action $action -Trigger $trigger -Settings $settings -Description "PROJECT ONE : resume horaire des depots suivis (lecture seule, en pause quand le jeu tourne)" -Force)
    Write-Output "Tache '$task' installee : une passe par heure, la premiere dans 2 minutes."
    exit 0
}

[void](New-Item -ItemType Directory -Force $Dossier)
$Dossier = (Resolve-Path $Dossier).Path
$logFile = Join-Path $Dossier "veille.log"
function Write-Log([string]$text) {
    if ((Test-Path $logFile) -and (Get-Item $logFile).Length -gt 256KB) { Move-Item -Force $logFile "$logFile.1" }
    Add-Content -Path $logFile -Value ("{0} {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $text) -Encoding UTF8
}

# Une seule passe a la fois (un verrou de plus de 15 minutes est un reste).
$lockFile = Join-Path $Dossier "veille.verrou"
if ((Test-Path $lockFile) -and ((Get-Date) - (Get-Item $lockFile).LastWriteTime).TotalMinutes -lt 15) { Write-Log "une passe tourne deja : rien fait"; exit 0 }
Set-Content -Path $lockFile -Value $PID
try {
    if (Get-Process victory -ErrorAction SilentlyContinue) { Write-Log "en pause : le jeu tourne, aucune requete"; exit 0 }
    if (-not (Test-Path $gh)) { Write-Log "gh introuvable : rien fait"; exit 1 }
    $sources = Join-Path $Dossier "suivi.txt"
    if (-not (Test-Path $sources)) { Write-Log "suivi.txt introuvable dans $Dossier : rien fait"; exit 1 }

    $deadline = (Get-Date).AddMinutes(4)
    $calls = 0
    # Lance gh avec une limite de temps ; rend sa sortie, ou rien.
    function Invoke-Gh([string[]]$Arguments, [int]$Seconds = 30) {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = $gh
        $psi.Arguments = ($Arguments | ForEach-Object { if ($_ -match '[\s"]') { '"' + ($_ -replace '"', '\"') + '"' } else { $_ } }) -join ' '
        $psi.RedirectStandardOutput = $true; $psi.RedirectStandardError = $true; $psi.UseShellExecute = $false; $psi.CreateNoWindow = $true
        $psi.StandardOutputEncoding = [Text.Encoding]::UTF8
        $p = [System.Diagnostics.Process]::Start($psi)
        $out = $p.StandardOutput.ReadToEndAsync()
        [void]$p.StandardError.ReadToEndAsync()
        if (-not $p.WaitForExit($Seconds * 1000)) { try { $p.Kill() } catch {}; return $null }
        $out.Result
    }
    # Une requete a l'API : statut, etiquette de version, corps. Une par seconde au plus.
    function Get-Api([string]$Path, [string]$Etag, [string]$Accept) {
        Start-Sleep -Milliseconds 1000
        $script:calls++
        $a = @('api', '-i', $Path)
        if ($Etag) { $a += @('-H', "If-None-Match: $Etag") }
        if ($Accept) { $a += @('-H', "Accept: $Accept") }
        $text = Invoke-Gh $a
        if ($null -eq $text) { return @{ status = 0; etag = $null; body = '' } }
        $status = 0; if ($text -match '^HTTP/[\d.]+ (\d+)') { $status = [int]$Matches[1] }
        $tag = $null; if ($text -match '(?im)^etag:\s*(.+?)\s*$') { $tag = $Matches[1] }
        $cut = $text.IndexOf("`r`n`r`n"); $skip = 4
        if ($cut -lt 0) { $cut = $text.IndexOf("`n`n"); $skip = 2 }
        $body = ''; if ($cut -ge 0) { $body = $text.Substring($cut + $skip) }
        @{ status = $status; etag = $tag; body = $body }
    }
    # Une liste JSON, element par element (PowerShell 5 rend le tableau d'un bloc).
    function ConvertFrom-JsonList([string]$text) {
        $parsed = $null
        try { $parsed = $text | ConvertFrom-Json } catch { return @() }
        @($parsed | ForEach-Object { $_ })
    }
    function ConvertTo-Table($object) {
        $table = @{}
        if ($object) { foreach ($p in $object.PSObject.Properties) { $table[$p.Name] = $p.Value } }
        $table
    }

    # Ce qui nous concerne, par poids : le texte d'un commit ou d'une demande est
    # note par les groupes de mots qu'il contient.
    $weights = @(
        @(5, 'bcpack', 'bc7', 'texture decode', 'deswizzle', 'detile', 'tiling'),
        @(4, 'residency', 'streaming', 'gpu memory', 'vram', 'budget', 'shared memory', 'out of memory', 'title memory'),
        @(3, 'pipeline', 'pso', 'shader translat', 'shader cache', 'precompil'),
        @(3, 'meshlet', 'culling', 'descriptor', 'bindless', 'mesh shader', 'indirect'),
        @(3, 'barycentric', 'rdna1', 'navi 10', 'navi10', '5600', '5700'),
        @(3, 'tdr', 'device removed', 'device lost', 'device hung', 'watchdog', 'page fault', 'use after free'),
        @(2, 'frame pacing', 'async', 'relay', 'fence', 'submit', 'latency')
    )
    function Get-Score([string]$text) {
        $low = $text.ToLowerInvariant()
        $score = 0; $words = @()
        foreach ($group in $weights) {
            $hit = $group[1..($group.Count - 1)] | Where-Object { $low.Contains($_) } | Select-Object -First 1
            if ($hit) { $score += $group[0]; $words += $hit }
        }
        if ($low -match '^(docs?|readme|typo)\b') { $score -= 2 }
        @{ score = $score; words = ($words -join ', ') }
    }

    $stateFile = Join-Path $Dossier "etat.json"
    $state = @{ depots = @{}; tracker = @{} }
    if (Test-Path $stateFile) {
        try {
            $read = Get-Content $stateFile -Raw -Encoding UTF8 | ConvertFrom-Json
            $state.tracker = ConvertTo-Table $read.tracker
            foreach ($p in $read.depots.PSObject.Properties) { $state.depots[$p.Name] = ConvertTo-Table $p.Value }
        } catch { Write-Log "etat.json illisible : repart de zero" }
    }

    $repos = New-Object System.Collections.Generic.List[string]
    $tracker = $null
    foreach ($line in (Get-Content $sources -Encoding UTF8)) {
        $parts = $line.Trim() -split '\s+'
        if ($parts[0] -eq 'repo' -and $parts.Count -ge 2 -and $parts[1] -match '^[\w.-]+/[\w.-]+$') { $repos.Add($parts[1]) }
        elseif ($parts[0] -eq 'tracker' -and $parts.Count -ge 3) { $tracker = @{ depot = $parts[1]; chemin = $parts[2] } }
    }

    $notes = New-Object System.Collections.Generic.List[string]
    # Le fichier du tracker : lu seulement s'il a change ; les depots qu'il nomme
    # s'ajoutent a ceux de suivi.txt (80 au plus en tout).
    if ($tracker) {
        $r = Get-Api ("repos/{0}/contents/{1}" -f $tracker.depot, $tracker.chemin) $state.tracker.etag "application/vnd.github.raw"
        if ($r.status -eq 200) {
            $state.tracker.etag = $r.etag
            $found = [regex]::Matches($r.body, 'github\.com/([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)') | ForEach-Object { $_.Groups[1].Value -replace '\.git$', '' } | Select-Object -Unique
            $state.tracker.depots = @($found)
            $notes.Add("Le fichier du tracker a change ({0} depots nommes)." -f @($found).Count)
        } elseif ($r.status -ne 304) { $notes.Add("Tracker illisible (statut {0})." -f $r.status) }
        foreach ($name in @($state.tracker.depots)) { if ($name -and -not $repos.Contains($name) -and $repos.Count -lt 80) { $repos.Add($name) } }
    }

    $candidates = New-Object System.Collections.Generic.List[object]
    $candidateFile = Join-Path $Dossier "candidats.json"
    if (Test-Path $candidateFile) { try { foreach ($c in (Get-Content $candidateFile -Raw -Encoding UTF8 | ConvertFrom-Json)) { $candidates.Add($c) } } catch {} }
    $fresh = New-Object System.Collections.Generic.List[object]
    $gone = New-Object System.Collections.Generic.List[string]
    $read = 0; $commits = 0; $pulls = 0; $stopped = $null
    $now = (Get-Date).ToUniversalTime()
    foreach ($repo in $repos) {
        if ((Get-Date) -gt $deadline) { $stopped = "limite de temps"; break }
        if (-not $state.depots.ContainsKey($repo)) { $state.depots[$repo] = @{} }
        $s = $state.depots[$repo]
        # La licence et l'existence du depot, une fois par jour.
        if (-not $s.licence_vue -or ([datetime]$s.licence_vue) -lt $now.AddHours(-24)) {
            $r = Get-Api "repos/$repo" $null $null
            if ($r.status -eq 404 -or $r.status -eq 451) {
                if (-not $s.disparu) { $gone.Add(("{0} (statut {1}, {2})" -f $repo, $r.status, $(if ($s.dernier_sha) { "derniere reference connue $($s.dernier_sha) du $($s.dernier_commit)" } else { "jamais lu ici" }))) }
                $s.disparu = $true; continue
            }
            if ($r.status -eq 403 -or $r.status -eq 429) { $stopped = "limite de GitHub (statut $($r.status))"; break }
            if ($r.status -eq 200) {
                try { $info = $r.body | ConvertFrom-Json; $s.licence = $(if ($info.license) { $info.license.spdx_id } else { 'aucune' }); $s.licence_vue = $now.ToString('o'); $s.disparu = $false } catch {}
            }
        }
        if ($s.disparu) { continue }
        $since = $(if ($s.dernier_commit) { ([datetime]$s.dernier_commit).ToUniversalTime().AddSeconds(1) } else { $now.AddHours(-24) })
        $r = Get-Api ("repos/{0}/commits?per_page=15&since={1}" -f $repo, $since.ToString('yyyy-MM-ddTHH:mm:ssZ')) $s.etag_commits $null
        if ($r.status -eq 403 -or $r.status -eq 429) { $stopped = "limite de GitHub (statut $($r.status))"; break }
        if ($r.status -eq 404 -or $r.status -eq 451) { if (-not $s.disparu) { $gone.Add("$repo (statut $($r.status))") }; $s.disparu = $true; continue }
        $read++
        if ($r.status -eq 200) {
            $s.etag_commits = $r.etag
            $list = ConvertFrom-JsonList $r.body
            foreach ($c in $list) {
                if (-not $c.sha) { continue }
                $commits++
                $title = ("$($c.commit.message)" -split "`n")[0]
                $score = Get-Score ("$($c.commit.message)".Substring(0, [Math]::Min(400, "$($c.commit.message)".Length)))
                $date = "$($c.commit.author.date)"
                if (-not $s.dernier_commit -or ([datetime]$date) -gt ([datetime]$s.dernier_commit)) { $s.dernier_commit = $date; $s.dernier_sha = $c.sha.Substring(0, 7) }
                if ($score.score -ge 3) {
                    $fresh.Add([pscustomobject]@{ vu = $now.ToString('o'); depot = $repo; type = 'commit'; ref = $c.sha.Substring(0, 7); auteur = $(if ($c.author) { $c.author.login } else { $c.commit.author.name }); date = $date; titre = $title; score = $score.score; mots = $score.words; licence = $s.licence; adresse = $c.html_url })
                }
            }
        }
        $r = Get-Api "repos/$repo/pulls?state=open&sort=updated&direction=desc&per_page=5" $s.etag_pulls $null
        if ($r.status -eq 200) {
            $s.etag_pulls = $r.etag
            $list = ConvertFrom-JsonList $r.body
            foreach ($q in $list) {
                if (-not $q.number -or ($s.derniere_demande -and ([datetime]$q.updated_at) -le ([datetime]$s.derniere_demande))) { continue }
                $pulls++
                $score = Get-Score "$($q.title)"
                if ($score.score -ge 3) {
                    $fresh.Add([pscustomobject]@{ vu = $now.ToString('o'); depot = $repo; type = 'demande'; ref = "#$($q.number)"; auteur = $q.user.login; date = "$($q.updated_at)"; titre = "$($q.title)"; score = $score.score; mots = $score.words; licence = $s.licence; adresse = $q.html_url })
                }
            }
            if ($list.Count -gt 0) { $s.derniere_demande = "$(($list | ForEach-Object { [datetime]$_.updated_at } | Sort-Object | Select-Object -Last 1).ToUniversalTime().ToString('o'))" }
        }
    }

    foreach ($c in $fresh) { $candidates.Add($c) }
    $kept = @($candidates | Select-Object -Last 500)
    ConvertTo-Json -InputObject $kept -Depth 4 | Set-Content -Path $candidateFile -Encoding UTF8
    $state.derniere_passe = $now.ToString('o')
    $state | ConvertTo-Json -Depth 6 | Set-Content -Path $stateFile -Encoding UTF8

    $digest = New-Object System.Collections.Generic.List[string]
    $digest.Add("# Veille du {0}" -f (Get-Date -Format "dd/MM/yyyy a HH:mm"))
    $digest.Add("")
    $digest.Add(("{0} depots lus sur {1}, {2} nouveaux commits, {3} demandes de fusion mises a jour, {4} requetes." -f $read, $repos.Count, $commits, $pulls, $calls))
    if ($stopped) { $digest.Add("Passe ecourtee : $stopped.") }
    foreach ($n in $notes) { $digest.Add($n) }
    foreach ($g in $gone) { $digest.Add("DISPARU : $g") }
    $digest.Add("")
    if ($fresh.Count -eq 0) { $digest.Add("Rien qui touche nos blocages.") }
    else {
        $digest.Add("Ce qui touche nos blocages (note, depot, reference, auteur, licence) :")
        foreach ($c in ($fresh | Sort-Object -Property score -Descending | Select-Object -First 20)) {
            $title = "$($c.titre)"; if ($title.Length -gt 90) { $title = $title.Substring(0, 90) }
            $digest.Add(("- {0} | {1} {2} | {3} | {4} | {5} [{6}]" -f $c.score, $c.depot, $c.ref, $c.auteur, $c.licence, $title, $c.mots))
        }
    }
    $digest.Add("")
    $digest.Add("Rien n'est telecharge ni fusionne : candidats dans candidats.json, a trancher.")
    $name = Join-Path $Dossier ((Get-Date -Format "yyyy-MM-dd_HH") + ".md")
    ($digest | Select-Object -First 30) | Set-Content -Path $name -Encoding UTF8
    Write-Log ("passe finie : {0} depots lus, {1} commits, {2} demandes, {3} candidats, {4} requetes{5}" -f $read, $commits, $pulls, $fresh.Count, $calls, $(if ($stopped) { ", ecourtee ($stopped)" } else { "" }))
} finally {
    Remove-Item $lockFile -ErrorAction SilentlyContinue
}

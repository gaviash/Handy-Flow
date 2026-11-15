# Stage 0 robustness scenarios on a Handy test copy (pwsh 7). Fictitious data only.
# Edits only the test copy's settings; restores them at the end.
param([string]$HandyDir, [string[]]$Only, [string]$Out)
$ErrorActionPreference = 'Stop'
Import-Module AudioDeviceCmdlets
$Dictate = Join-Path $PSScriptRoot 'dictate.ps1'
$Store = Join-Path $HandyDir 'Data\settings_store.json'
$Backup = Get-Content $Store -Raw -Encoding utf8
function Log($o) { $o | Format-List | Out-String -Width 220 | Write-Host; if ($Out) { ($o | ConvertTo-Json -Compress -Depth 5) | Add-Content $Out -Encoding utf8 } }
function Restart-Handy {
  Get-Process handy -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep 2
  Start-Process (Join-Path $HandyDir 'handy.exe') -ArgumentList '--start-hidden'; Start-Sleep 10
}
function Edit-Settings([scriptblock]$f) {
  Get-Process handy -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep 2
  $d = Get-Content $Store -Raw -Encoding utf8 | ConvertFrom-Json; & $f $d.settings
  [IO.File]::WriteAllText($Store, ($d | ConvertTo-Json -Depth 20), [Text.UTF8Encoding]::new($false)); Restart-Handy
}
function Log-Tail([string]$pattern, [int]$n = 3) {
  $f = Get-ChildItem (Join-Path $HandyDir 'Data\logs\*.log') | Sort-Object LastWriteTime | Select-Object -Last 1
  @(Select-String -Path $f -Pattern $pattern | Select-Object -Last $n | ForEach-Object { $_.Line.Substring(0, [Math]::Min(170, $_.Line.Length)) })
}
function Want($name) { -not $Only -or $Only -contains $name }

try {
  if (Want 'S08') {
    # Groq unavailable: invalid key, then an unroutable endpoint (connect hangs past the 8 s deadline).
    foreach ($case in 'badkey', 'timeout') {
      Edit-Settings { param($s)
        $s.post_process_api_keys.groq = if ($case -eq 'badkey') { 'gsk_invalid_bench_key' } else { ($Backup | ConvertFrom-Json).settings.post_process_api_keys.groq }
        ($s.post_process_providers | Where-Object id -eq 'groq').base_url = if ($case -eq 'timeout') { 'https://10.255.255.1/openai/v1' } else { 'https://api.groq.com/openai/v1' }
      }
      foreach ($r in & $Dictate -HandyDir $HandyDir -Ids F06, F01, F03, GE09, S27g -WaitMs 20000) {
        Log ([ordered]@{ scenario = "S08-$case"; id = $r.id; pasted = [bool]$r.field; fieldIsCleanedRaw = ($r.field -eq $r.raw); post = $r.post; ms = $r.releaseToText })
      }
      Log ([ordered]@{ scenario = "S08-$case-log"; lines = (Log-Tail 'Rewrite failed|timed out|Post-processing' 2) })
    }
    [IO.File]::WriteAllText($Store, $Backup, [Text.UTF8Encoding]::new($false)); Restart-Handy
  }
  if (Want 'S34') {
    # Injected keys (letters, Enter, Delete, Shift) between two dictations: the second paste must succeed.
    Add-Type -AssemblyName System.Windows.Forms
    foreach ($keys in 'abc', '{ENTER}', '{DELETE}', '+') {
      $first = & $Dictate -HandyDir $HandyDir -Ids F06 -KeepPad
      $pad = Get-Process | Where-Object { $_.Path -like "$env:TEMP\handy-pads\*" } | Select-Object -First 1
      if (-not $pad -or [HB]::FgPid() -ne $pad.Id) { throw 'S34: TestPad not foreground, refusing to inject keys' }
      [System.Windows.Forms.SendKeys]::SendWait($keys); Start-Sleep -Milliseconds 300
      $second = & $Dictate -HandyDir $HandyDir -Ids F01
      Log ([ordered]@{ scenario = 'S34'; injected = $keys; firstPasted = [bool]$first.field; secondPasted = [bool]$second.field })
    }
  }
  if (Want 'S48') {
    foreach ($m in 'escListen', 'escProcess') {
      foreach ($r in & $Dictate -HandyDir $HandyDir -Ids F06, F01, F03 -Mode $m -WaitMs 8000) {
        Log ([ordered]@{ scenario = "S48-$m"; id = $r.id; nothingPasted = -not $r.field; historyRaw = $r.raw })
      }
    }
  }
  if (Want 'S49') {
    foreach ($r in & $Dictate -HandyDir $HandyDir -Ids F06, F01, F03 -Mode toggle) {
      Log ([ordered]@{ scenario = 'S49-toggle'; id = $r.id; pasted = [bool]$r.field; ms = $r.releaseToText })
    }
    # Accidental short press, nothing said: what does hold_or_toggle do?
    $r = & $Dictate -HandyDir $HandyDir -Ids F06 -Mode tap -WaitMs 5000 -KeepPad
    $rec = Log-Tail 'start(ed)? recording|Recording started|stop(ped)? recording|TranscribeAction::start' 4
    Log ([ordered]@{ scenario = 'S49-tap'; nothingPasted = -not $r.field; log = $rec })
    # Second short press ends the hands-free recording that the tap started, on silence.
    $r2 = & $Dictate -HandyDir $HandyDir -Ids F06 -Mode tap -WaitMs 8000
    Log ([ordered]@{ scenario = 'S49-tap-stop'; nothingPasted = -not $r2.field; historyRaw = $r2.raw })
  }
  if (Want 'S32') {
    # Clipboard restored as it was (text, image, rich), and no dictation in the Win+V history.
    $Clip = { param($op, $v = '') powershell -NoProfile -STA -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'clip.ps1') $op $v }
    $ids = @('F05', 'F07', 'S27c')
    foreach ($kind in 'text', 'image', 'html') {
      $id = $ids[[array]::IndexOf(@('text', 'image', 'html'), $kind)]
      & $Clip "set-$kind" "S32-orig-$kind" | Out-Null
      $before = & $Clip get | ConvertFrom-Json
      $r = & $Dictate -HandyDir $HandyDir -Ids $id
      Start-Sleep 2
      $after = & $Clip get | ConvertFrom-Json
      $hist = & $Clip history | ConvertFrom-Json
      Log ([ordered]@{ scenario = "S32-$kind"; pasted = [bool]$r.field; restored = (($before | ConvertTo-Json -Compress) -eq ($after | ConvertTo-Json -Compress)); before = $before; after = $after
        inWinV = [bool]($hist.texts | Where-Object { $r.field -and $_ -eq $r.field }) })
    }
    # Timeout path: the target never reads the transcript.
    & $Clip set-text 'S32-orig-timeout' | Out-Null
    $r = & $Dictate -HandyDir $HandyDir -Ids F04 -NoReadFault -WaitMs 12000
    Start-Sleep 2
    $after = & $Clip get | ConvertFrom-Json; $hist = & $Clip history | ConvertFrom-Json
    Log ([ordered]@{ scenario = 'S32-timeout'; pasted = [bool]$r.field; restored = ($after.text -eq 'S32-orig-timeout'); post = $r.post
      inWinV = [bool]($hist.texts | Where-Object { $_ -and $r.post -and $_.Trim() -eq $r.post.Trim() }); log = (Log-Tail 'reliable-paste' 3) })
  }
  if (Want 'S51') {
    # "Default" microphone follows a change of the Windows default, without restarting Handy.
    $recRestore = Get-AudioDevice -Recording
    $cableOut = Get-AudioDevice -List | Where-Object { $_.Type -eq 'Recording' -and $_.Name -like 'CABLE Output*' } | Select-Object -First 1
    $other = Get-AudioDevice -List | Where-Object { $_.Type -eq 'Recording' -and $_.Name -notlike 'CABLE*' } | Select-Object -First 1
    try {
      Set-AudioDevice -ID $other.ID | Out-Null
      Edit-Settings { param($s) $s.selected_microphone = $null }
      Set-AudioDevice -ID $cableOut.ID | Out-Null; Start-Sleep 2   # changed while Handy runs
      $a = & $Dictate -HandyDir $HandyDir -Ids F06
      Set-AudioDevice -ID $other.ID | Out-Null; Start-Sleep 2      # back to a mic that hears no bench audio
      $b = & $Dictate -HandyDir $HandyDir -Ids F01 -WaitMs 8000
      Log ([ordered]@{ scenario = 'S51'; followedToCable = [bool]$a.field; followedAway = ($b.field -ne 'Le rapport trimestriel est prêt, il manque seulement les chiffres du mois de septembre.'); otherMic = $other.Name; b = $b.field })
    } finally { Set-AudioDevice -ID $recRestore.ID | Out-Null; [IO.File]::WriteAllText($Store, $Backup, [Text.UTF8Encoding]::new($false)) }
  }
} finally {
  [IO.File]::WriteAllText($Store, $Backup, [Text.UTF8Encoding]::new($false))
}

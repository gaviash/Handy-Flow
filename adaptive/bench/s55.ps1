# S55 non-regression: transcribe every WAV of -AudioDirs with -Exe (headless
# --transcribe-file) into -Out (jsonl). With -Compare, diff against that file.
param([string]$Exe, [string[]]$AudioDirs, [string]$Out, [string]$Compare)
Remove-Item $Out -ErrorAction SilentlyContinue
foreach ($w in ($AudioDirs | ForEach-Object { Get-ChildItem $_ -Filter *.wav } | Sort-Object Name)) {
  $o = Join-Path $env:TEMP "s55_$PID.txt"
  Start-Process $Exe -ArgumentList "--transcribe-file `"$($w.FullName)`" --json" -RedirectStandardOutput $o -RedirectStandardError "$o.err" -Wait -NoNewWindow
  $r = Get-Content $o -Raw | ConvertFrom-Json
  [pscustomobject]@{ file = $w.Name; text = $r.text; ms = $r.best_ms; backend = $r.bound_backend } | ConvertTo-Json -Compress | Add-Content $Out -Encoding utf8
}
if ($Compare) {
  $ref = @{}; Get-Content $Compare -Encoding utf8 | ForEach-Object { $j = $_ | ConvertFrom-Json; $ref[$j.file] = $j.text }
  $diff = 0; $n = 0
  Get-Content $Out -Encoding utf8 | ForEach-Object { $j = $_ | ConvertFrom-Json; $n++; if ($ref[$j.file] -ne $j.text) { $diff++; "DIFF $($j.file)" } }
  "S55: $($n - $diff)/$n identical"
}

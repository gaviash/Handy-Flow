# Generates the fictional bench corpus as 16 kHz mono WAVs (SAPI voices).
param([string]$OutDir = (Join-Path $env:TEMP 'handy_bench_audio'))
Add-Type -AssemblyName System.Speech
New-Item -ItemType Directory -Force $OutDir | Out-Null
$voices = @{ fr = 'Microsoft Hortense Desktop'; en = 'Microsoft Zira Desktop' }
$fmt = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono)
foreach ($c in (Get-Content (Join-Path $PSScriptRoot 'corpus.json') -Raw -Encoding utf8 | ConvertFrom-Json)) {
  $s = New-Object System.Speech.Synthesis.SpeechSynthesizer
  $s.SelectVoice($voices[$c.lang]); $s.SetOutputToWaveFile((Join-Path $OutDir "$($c.id).wav"), $fmt)
  $s.Speak($c.text); $s.Dispose()
}
Get-ChildItem $OutDir -Filter *.wav | Measure-Object | ForEach-Object { "$($_.Count) wav in $OutDir" }

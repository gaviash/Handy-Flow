# Stage 1 context and privacy scenarios (pwsh 7): S22/J09, S09/J11a/J11b
# (and the release-in-password variant), S17, S16, S06, F33 exclusion, S21.
# The Handy under test runs from -HandyDir with CABLE Output as microphone and
# its Groq base URL pointed at mock_llm.py, whose log shows what was sent.
# Fictitious data only.
param(
  [Parameter(Mandatory)][string]$HandyDir,
  [Parameter(Mandatory)][string]$MockLog,
  [string]$Out,
  [string]$AudioDir = (Join-Path $env:TEMP 'handy_bench_audio'),
  [string[]]$Only
)
Import-Module AudioDeviceCmdlets
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms
$AE = [System.Windows.Automation.AutomationElement]; $TS = [System.Windows.Automation.TreeScope]
if (-not ('HB' -as [type])) {
  Add-Type @'
using System; using System.Runtime.InteropServices;
public static class HB {
  [DllImport("user32.dll")] static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] static extern uint MapVirtualKey(uint code, uint type);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  public static void Down(byte vk){ keybd_event(vk,(byte)MapVirtualKey(vk,0),0,UIntPtr.Zero); }
  public static void Up(byte vk){ keybd_event(vk,(byte)MapVirtualKey(vk,0),2,UIntPtr.Zero); }
  public static uint FgPid(){ uint p; GetWindowThreadProcessId(GetForegroundWindow(), out p); return p; }
  public static bool Bring(IntPtr h){ keybd_event(0x12,0,0,UIntPtr.Zero); keybd_event(0xE8,0,0,UIntPtr.Zero); keybd_event(0xE8,0,2,UIntPtr.Zero); bool ok = SetForegroundWindow(h); keybd_event(0x12,0,2,UIntPtr.Zero); return ok; }
}
'@
}
$Chrome = "$env:ProgramFiles\Google\Chrome\Application\chrome.exe"
$PadSrc = Join-Path $PSScriptRoot '..\..\..\wispr_insights\fixtures\testpad\TestPad.exe'
$PadDir = Join-Path $env:TEMP 'handy-pads'; New-Item -ItemType Directory -Force $PadDir | Out-Null
$Page = Join-Path $env:TEMP 'ctx_testpage.html'
(Get-Content (Join-Path $PSScriptRoot '..\..\..\wispr_insights\probes\testpage.html') -Raw -Encoding utf8) -replace '</body>',
  '<script>setTimeout(()=>{const e=document.getElementById(location.hash.slice(1));if(e){e.focus();if(e.isContentEditable){const r=document.createRange();r.setStart(e.lastChild,8);r.collapse(true);getSelection().removeAllRanges();getSelection().addRange(r);}}},300)</script></body>' |
  Set-Content $Page -Encoding utf8
$Db = Join-Path $HandyDir 'Data\history.db'
$Rec = Join-Path $HandyDir 'Data\recordings'
$cable = Get-AudioDevice -List | Where-Object { $_.Type -eq 'Playback' -and $_.Name -like 'CABLE Input*' } | Select-Object -First 1

function Last-Id { [int](node -e 'const {DatabaseSync}=require("node:sqlite");const d=new DatabaseSync(process.argv[1],{readOnly:true});console.log((d.prepare("select max(id) m from transcription_history").get()||{}).m||0)' $Db 2>$null) }
function Mock-Count { if (Test-Path $MockLog) { @(Get-Content $MockLog -Encoding utf8).Count } else { 0 } }
function Mock-Since([int]$n) { if (Test-Path $MockLog) { @(Get-Content $MockLog -Encoding utf8 | Select-Object -Skip $n | ForEach-Object { ($_ | ConvertFrom-Json).messages[-1].content }) } else { @() } }
function Value($el) { $o = $null; if ($el -and $el.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$o)) { return $o.Current.Value }
  if ($el -and $el.TryGetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern, [ref]$o)) { return $o.DocumentRange.GetText(-1) } }
function ByName($root, [string]$name) { $root.FindFirst($TS::Descendants, (New-Object System.Windows.Automation.PropertyCondition($AE::NameProperty, $name))) }

function Open-Page([string]$id) {
  Get-Process chrome -ErrorAction SilentlyContinue | Out-Null
  Start-Process $Chrome -ArgumentList '--new-window', "file:///$($Page -replace '\\','/')#$id"
  Start-Sleep -Milliseconds 2500
  $w = $AE::FromHandle([HB]::GetForegroundWindow())
  return $w
}
function Close-Fg { [HB]::Down(0x11); [HB]::Down(0x57); [HB]::Up(0x57); [HB]::Up(0x11); Start-Sleep -Milliseconds 500 }  # Ctrl+W
function Start-Pad([string]$name, [string]$text = '') {
  Get-Process | Where-Object { $_.Path -like "$PadDir\*" } | Stop-Process -Force -ErrorAction SilentlyContinue; Start-Sleep -Milliseconds 300
  $bin = Join-Path $PadDir "$name.exe"; if (-not (Test-Path $bin)) { Copy-Item $PadSrc $bin }
  $tf = Join-Path $PadDir 'pad.txt'; [IO.File]::WriteAllText($tf, $text, [Text.UTF8Encoding]::new($false))
  $p = Start-Process $bin -ArgumentList @("Pad-$name", "`"$tf`"", $text.Length, 0) -PassThru
  for ($i = 0; $i -lt 40 -and $p.MainWindowHandle -eq 0; $i++) { Start-Sleep -Milliseconds 100; $p.Refresh() }
  Start-Sleep -Milliseconds 300; if ([HB]::FgPid() -ne $p.Id) { [HB]::Bring($p.MainWindowHandle) | Out-Null; Start-Sleep -Milliseconds 200 }
  return $p
}
function Pad-Field($p) {
  $w = $AE::FromHandle($p.MainWindowHandle)
  foreach ($ct in 'Document', 'Edit') {
    $el = $w.FindFirst($TS::Descendants, (New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::$ct)))
    if ($el) { return $el }
  }
}

# Push-to-talk dictation of one corpus WAV; $during runs after speech, before release.
function Dictate([string]$wav, [scriptblock]$during) {
  $restore = Get-AudioDevice -Playback
  try {
    Set-AudioDevice -ID $cable.ID | Out-Null; Start-Sleep -Milliseconds 250
    [HB]::Down(0x11); [HB]::Down(0x20); Start-Sleep -Milliseconds 600
    (New-Object System.Media.SoundPlayer (Join-Path $AudioDir "$wav.wav")).PlaySync()
    Start-Sleep -Milliseconds 400
    if ($during) { & $during; Start-Sleep -Milliseconds 300 }
    [HB]::Up(0x20); [HB]::Up(0x11)
  } finally { Set-AudioDevice -ID $restore.ID | Out-Null }
}
function Wait-Until([scriptblock]$cond, [int]$ms = 15000) { $sw = [Diagnostics.Stopwatch]::StartNew(); while ($sw.ElapsedMilliseconds -lt $ms) { if (& $cond) { return $true }; Start-Sleep -Milliseconds 100 }; return $false }

$results = @()
function Record([string]$id, [bool]$pass, $detail) {
  $r = [ordered]@{ id = $id; pass = $pass; detail = $detail }
  if ($Out) { ($r | ConvertTo-Json -Compress -Depth 5) | Add-Content $Out -Encoding utf8 }
  $script:results += [pscustomobject]$r; [pscustomobject]$r | Format-List | Out-String | Write-Host
}
function Want([string]$id) { -not $Only -or $Only -contains $id }

if (Want 'S22') {
  $w = Open-Page 'f_ph'; $el = ByName $w 'Vide avec placeholder'; $m = Mock-Count
  Dictate 'CT02'; $ok = Wait-Until { Value $el }; Start-Sleep -Milliseconds 500
  $v = Value $el; $sent = Mock-Since $m
  Record 'S22' ($ok -and $v -and -not $v.StartsWith(' ') -and $sent.Count -eq 1 -and $sent[0] -notmatch 'before cursor') @{ field = $v; sent = $sent }
  Close-Fg
}
if (Want 'S09') {
  $w = Open-Page 'f_pw'; $el = ByName $w 'Mot de passe'; $len0 = (Value $el).Length
  $m = Mock-Count; $h = Last-Id; $wavs = @(Get-ChildItem $Rec).Count
  Dictate 'CT01'; $ok = Wait-Until { (Value $el).Length -gt $len0 }; Start-Sleep -Milliseconds 2500
  $grew = (Value $el).Length - $len0
  Record 'S09' ($ok -and (Mock-Count) -eq $m -and (Last-Id) -eq $h -and @(Get-ChildItem $Rec).Count -eq $wavs) @{ pasted_chars = $grew; sent = (Mock-Count) - $m; history = (Last-Id) - $h; wavs = @(Get-ChildItem $Rec).Count - $wavs }
  Close-Fg
}
if (Want 'S09b') {
  # Starts in a normal field, released in the password field.
  $w = Open-Page 'f_input'; $pw = ByName $w 'Mot de passe'; $len0 = (Value $pw).Length
  $m = Mock-Count; $h = Last-Id
  Dictate 'CT01' { $pw.SetFocus() }; $ok = Wait-Until { (Value $pw).Length -gt $len0 }; Start-Sleep -Milliseconds 2500
  Record 'S09b' ($ok -and (Mock-Count) -eq $m -and (Last-Id) -eq $h) @{ pasted_chars = (Value $pw).Length - $len0; sent = (Mock-Count) - $m; history = (Last-Id) - $h }
  Close-Fg
}
if (Want 'S17') {
  $w = Open-Page 'f_rich'; $el = ByName $w 'Editeur riche'; $v0 = Value $el; $m = Mock-Count
  Dictate 'CT03'; $ok = Wait-Until { (Value $el) -ne $v0 }; Start-Sleep -Milliseconds 500
  $sent = @(Mock-Since $m); $s = $sent -join "`n"
  $leak = @('Brouillon de test', 'Réduire', 'Agrandir', 'Fermer', 'Mot de passe', '•', '@example.com', 'Nouvel onglet', 'Signets', 'Favoris') | Where-Object { $s.Contains($_) }
  Record 'S17' ($ok -and $sent.Count -eq 1 -and -not $leak -and $s -match 'Bérénice Kowalczyk' -and $s -match 'text before cursor: «Bonjour Bérénice, voici »') @{ leaks = $leak; sent = $sent; field = (Value $el) }
  Close-Fg
}
if (Want 'S16') {
  $w = Open-Page 'f_rich'; Start-Sleep -Milliseconds 500
  $pad = Start-Pad 'notepad'; $f = Pad-Field $pad; $m = Mock-Count
  Dictate 'CT04'; $ok = Wait-Until { Value $f }; Start-Sleep -Milliseconds 500
  $s = (Mock-Since $m) -join "`n"
  Record 'S16' ($ok -and $s -match 'category: document' -and $s -notmatch 'names:|Bérénice|Kowalczyk|Ksenia') @{ sent = $s; field = (Value $f) }
  Stop-Process -Id $pad.Id -Force; [HB]::Bring([HB]::GetForegroundWindow()) | Out-Null; Close-Fg
}
if (Want 'S06') {
  # Starts on the page with names, switches to an AI app while speaking, released there.
  $pad = Start-Pad 'chatgpt'; $f = Pad-Field $pad
  $w = Open-Page 'f_rich'; $m = Mock-Count
  Dictate 'CT04' { [HB]::Bring($pad.MainWindowHandle) | Out-Null }
  $ok = Wait-Until { Value $f }; Start-Sleep -Milliseconds 500
  $s = (Mock-Since $m) -join "`n"
  Record 'S06' ($ok -and $s -match 'category: ai' -and $s -notmatch 'names:|Bérénice|Kowalczyk') @{ sent = $s; field = (Value $f) }
  Stop-Process -Id $pad.Id -Force; Start-Sleep -Milliseconds 300
  $cw = Get-Process chrome | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1; if ($cw) { [HB]::Bring($cw.MainWindowHandle) | Out-Null; Close-Fg }
}
if (Want 'F33') {
  $pad = Start-Pad 'keepassxc'; $f = Pad-Field $pad; $m = Mock-Count; $h = Last-Id
  Dictate 'CT04'; $saved = Wait-Until { (Last-Id) -gt $h }; Start-Sleep -Milliseconds 2000
  $s = (Mock-Since $m) -join "`n"
  Record 'F33' ($saved -and -not (Value $f) -and $s -notmatch '<context>') @{ field = (Value $f); sent = $s; history = (Last-Id) - $h }
  Stop-Process -Id $pad.Id -Force
}
if (Want 'S21') {
  # A window whose UI thread is blocked: the dictation must finish without context and the next one work.
  $hung = Start-Process pwsh -ArgumentList '-NoProfile', '-Command', 'Add-Type -AssemblyName System.Windows.Forms; $f = New-Object Windows.Forms.Form; $f.Text = "Hung"; $t = New-Object Windows.Forms.TextBox; $f.Controls.Add($t); $f.Add_Shown({ $t.Focus(); [Windows.Forms.Application]::DoEvents(); Start-Sleep 25 }); [Windows.Forms.Application]::Run($f)' -PassThru
  Start-Sleep -Seconds 3; $h = Last-Id; $sw = [Diagnostics.Stopwatch]::StartNew()
  Dictate 'CT04'; $up = $sw.ElapsedMilliseconds; $saved = Wait-Until { (Last-Id) -gt $h } 20000; $done = $sw.ElapsedMilliseconds - $up
  Stop-Process -Id $hung.Id -Force -ErrorAction SilentlyContinue; Start-Sleep -Seconds 2
  $pad = Start-Pad 'testpad'; $f = Pad-Field $pad
  Dictate 'CT02'; $next = Wait-Until { Value $f }
  Record 'S21' ($saved -and $next) @{ history_after_release_ms = $done; next_field = (Value $f) }
  Stop-Process -Id $pad.Id -Force
}
$results | Format-Table id, pass -AutoSize | Out-String | Write-Host

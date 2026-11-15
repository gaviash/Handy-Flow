# Real dictation bench (pwsh 7): TestPad target, corpus WAV played into VB-Cable
# while the shortcut is held, text timed in the field via UIA, history read back.
# The Handy under test must run from -HandyDir with CABLE Output as microphone.
# Fictitious data only. Adapted from wispr_insights/fixtures/harness/run3.ps1.
param(
  [string]$HandyDir,
  [string[]]$Ids,
  [string]$Out,
  [ValidateSet('main', 'raw')][string]$Shortcut = 'main',
  [string]$AudioDir = (Join-Path $env:TEMP 'handy_bench_audio'),
  [string]$PadExe = 'testpad',
  [int]$Repeat = 1,
  [int]$WaitMs = 15000,
  [int]$PollMs = 8,
  [switch]$KeepPad,
  # Fault injection: a lone Alt tap puts TestPad in menu mode, so the paste is never read.
  [switch]$NoReadFault,
  # hold: push-to-talk. toggle: short press to start and to stop (hands-free).
  # escListen: hands-free start, then Escape alone while listening (holding the
  # chord would send Ctrl+Esc). escProcess: Escape right after release (S48).
  # tap: one accidental short press, nothing spoken (S49).
  [ValidateSet('hold', 'toggle', 'escListen', 'escProcess', 'tap')][string]$Mode = 'hold'
)
Import-Module AudioDeviceCmdlets
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$AE = [System.Windows.Automation.AutomationElement]; $TS = [System.Windows.Automation.TreeScope]
$PadSrc = Join-Path $PSScriptRoot '..\..\..\wispr_insights\fixtures\testpad\TestPad.exe'
$PadDir = Join-Path $env:TEMP 'handy-pads'; New-Item -ItemType Directory -Force $PadDir | Out-Null
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
  // A lone Alt tap puts the target in menu mode, which swallows the next Ctrl+V:
  // mask it with the inert VK 0xE8 between Alt down and up.
  public static bool Bring(IntPtr h){ keybd_event(0x12,0,0,UIntPtr.Zero); keybd_event(0xE8,0,0,UIntPtr.Zero); keybd_event(0xE8,0,2,UIntPtr.Zero); bool ok = SetForegroundWindow(h); keybd_event(0x12,0,2,UIntPtr.Zero); return ok; }
}
'@
}
$Keys = if ($Shortcut -eq 'main') { @(0x11, 0x20) } else { @(0x11, 0x10, 0x20) }  # Ctrl+Space / Ctrl+Shift+Space
$Db = Join-Path $HandyDir 'Data\history.db'

function Get-LastRow {
  $js = 'const {DatabaseSync}=require("node:sqlite");const d=new DatabaseSync(process.argv[1],{readOnly:true});' +
        'console.log(JSON.stringify(d.prepare("select id,transcription_text raw,post_processed_text post,post_process_requested req from transcription_history order by id desc limit 1").get()||{}))'
  return (node -e $js $Db 2>$null | ConvertFrom-Json)
}
function Start-Pad([string]$text = '') {
  Get-Process | Where-Object { $_.Path -like "$PadDir\*" } | Stop-Process -Force -ErrorAction SilentlyContinue; Start-Sleep -Milliseconds 300
  $bin = Join-Path $PadDir "$PadExe.exe"; if (-not (Test-Path $bin)) { Copy-Item $PadSrc $bin }
  $tf = Join-Path $PadDir 'pad.txt'; [IO.File]::WriteAllText($tf, $text, [Text.UTF8Encoding]::new($false))
  $p = Start-Process $bin -ArgumentList @("Pad-$PadExe", "`"$tf`"", $text.Length, 0) -PassThru
  for ($i = 0; $i -lt 40 -and $p.MainWindowHandle -eq 0; $i++) { Start-Sleep -Milliseconds 100; $p.Refresh() }
  Start-Sleep -Milliseconds 300; if ([HB]::FgPid() -ne $p.Id) { [HB]::Bring($p.MainWindowHandle) | Out-Null; Start-Sleep -Milliseconds 200 }
  if ([HB]::FgPid() -ne $p.Id) { throw "pad not foreground" }
  return $p
}
function Get-Field($p) {
  $w = $AE::FromHandle($p.MainWindowHandle)
  foreach ($ct in 'Document', 'Edit') {
    $el = $w.FindFirst($TS::Descendants, (New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::$ct)))
    if ($el) { return $el }
  }
}
function Read-Field($el) {
  $o = $null
  if ($el.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$o)) { return $o.Current.Value }
  if ($el.TryGetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern, [ref]$o)) { return $o.DocumentRange.GetText(-1) }
}

$corpus = Get-Content (Join-Path $PSScriptRoot 'corpus.json') -Raw -Encoding utf8 | ConvertFrom-Json
if ($Ids) { $corpus = $corpus | Where-Object { $Ids -contains $_.id } }
$cable = Get-AudioDevice -List | Where-Object { $_.Type -eq 'Playback' -and $_.Name -like 'CABLE Input*' } | Select-Object -First 1
foreach ($rep in 1..$Repeat) {
  foreach ($c in $corpus) {
    $pad = Start-Pad; $field = Get-Field $pad; $before = Get-LastRow
    if ($NoReadFault) { [HB]::Down(0x12); [HB]::Up(0x12); Start-Sleep -Milliseconds 200 }
    $restore = Get-AudioDevice -Playback; $sw = [Diagnostics.Stopwatch]::StartNew(); $t = [ordered]@{}
    try {
      Set-AudioDevice -ID $cable.ID | Out-Null; Start-Sleep -Milliseconds 250
      $press = { $Keys | ForEach-Object { [HB]::Down($_) } }
      $release = { for ($k = $Keys.Count - 1; $k -ge 0; $k--) { [HB]::Up($Keys[$k]) } }
      $esc = { [HB]::Down(0x1B); Start-Sleep -Milliseconds 40; [HB]::Up(0x1B) }
      if ($Mode -in 'toggle', 'tap', 'escListen') { & $press; Start-Sleep -Milliseconds 80; & $release } else { & $press }
      Start-Sleep -Milliseconds 600
      if ($Mode -ne 'tap') { (New-Object System.Media.SoundPlayer (Join-Path $AudioDir "$($c.id).wav")).PlaySync() }
      Start-Sleep -Milliseconds 500
      if ($Mode -eq 'escListen') { & $esc; Start-Sleep -Milliseconds 200 }
      if ($Mode -eq 'toggle') { & $press; Start-Sleep -Milliseconds 80; & $release } elseif ($Mode -notin 'tap', 'escListen') { & $release }
      $t.up = $sw.ElapsedMilliseconds
      if ($Mode -eq 'escProcess') { Start-Sleep -Milliseconds 150; & $esc }
    } finally { Set-AudioDevice -ID $restore.ID | Out-Null }
    while ($sw.ElapsedMilliseconds -lt $t.up + $WaitMs) { if (Read-Field $field) { $t.text = $sw.ElapsedMilliseconds; break }; Start-Sleep -Milliseconds $PollMs }
    Start-Sleep -Milliseconds 400
    $row = $null; for ($i = 0; $i -lt 20; $i++) { $row = Get-LastRow; if ($row.id -ne $before.id) { break }; Start-Sleep -Milliseconds 250 }
    $r = [ordered]@{ id = $c.id; rep = $rep; shortcut = $Shortcut; field = (Read-Field $field)
      releaseToText = if ($t.text) { $t.text - $t.up } else { $null }
      raw = if ($row.id -ne $before.id) { $row.raw } else { $null }; post = $row.post; fg = ([HB]::FgPid() -eq $pad.Id) }
    if ($Out) { ($r | ConvertTo-Json -Compress) | Add-Content $Out -Encoding utf8 }
    [pscustomobject]$r
    if (-not $KeepPad) { Stop-Process -Id $pad.Id -Force -ErrorAction SilentlyContinue }
  }
}

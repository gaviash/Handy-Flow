# Clipboard helper for S32 (Windows PowerShell, STA). set-text|set-image|set-html|get|history
param([string]$Op, [string]$Value = '')
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
switch ($Op) {
  'set-text' { [System.Windows.Forms.Clipboard]::SetText($Value) }
  'set-image' { $b = New-Object System.Drawing.Bitmap 7, 5; $b.SetPixel(3, 2, [System.Drawing.Color]::Red); [System.Windows.Forms.Clipboard]::SetImage($b) }
  'set-html' {
    $d = New-Object System.Windows.Forms.DataObject
    $html = "Version:0.9`r`nStartHTML:00000097`r`nEndHTML:00000170`r`nStartFragment:00000131`r`nEndFragment:00000134`r`n<html><body><!--StartFragment--><b>$Value</b><!--EndFragment--></body></html>"
    $d.SetData([System.Windows.Forms.DataFormats]::Html, $html); $d.SetData([System.Windows.Forms.DataFormats]::UnicodeText, $Value)
    [System.Windows.Forms.Clipboard]::SetDataObject($d, $true)
  }
  'get' {
    $o = [System.Windows.Forms.Clipboard]::GetDataObject()
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    [pscustomobject]@{ text = [System.Windows.Forms.Clipboard]::GetText(); image = if ($img) { "$($img.Width)x$($img.Height)" } else { $null }
      html = [bool]$o.GetDataPresent([System.Windows.Forms.DataFormats]::Html); formats = ($o.GetFormats() -join ',') } | ConvertTo-Json -Compress
  }
  'history' {
    # Win+V history: texts currently listed (needs clipboard history enabled).
    $enabled = (Get-ItemProperty 'HKCU:\Software\Microsoft\Clipboard' -ErrorAction SilentlyContinue).EnableClipboardHistory
    [Windows.ApplicationModel.DataTransfer.Clipboard, Windows.ApplicationModel.DataTransfer, ContentType = WindowsRuntime] | Out-Null
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $asTask = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' } | Select-Object -First 1
    $await = { param($op, $t) $task = $asTask.MakeGenericMethod($t).Invoke($null, @($op)); $task.Wait(5000) | Out-Null; $task.Result }
    $res = & $await ([Windows.ApplicationModel.DataTransfer.Clipboard]::GetHistoryItemsAsync()) ([Windows.ApplicationModel.DataTransfer.ClipboardHistoryItemsResult])
    $texts = foreach ($it in $res.Items) { try { & $await ($it.Content.GetTextAsync()) ([string]) } catch {} }
    [pscustomobject]@{ enabled = $enabled; status = "$($res.Status)"; texts = @($texts) } | ConvertTo-Json -Compress
  }
}

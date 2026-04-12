# SPDX-License-Identifier: MIT
$repo = "ininids/aam"
$assetName = "aam-windows-amd64.exe"
$url = (Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest").assets | Where-Object { $_.name -eq $assetName } | Select-Object -ExpandProperty browser_download_url

Invoke-WebRequest -Uri $url -OutFile "$HOME\AppData\Local\Microsoft\WindowsApps\aam.exe"
Write-Host "Success! aam installed to WindowsApps folder."
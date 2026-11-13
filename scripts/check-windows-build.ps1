$ErrorActionPreference = 'Stop'
$binary = Join-Path $PSScriptRoot '..\target\release\wifinx-validator.exe'
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$dumpbin = & $vswhere -latest -products '*' -find 'VC\Tools\MSVC\**\bin\Hostx64\x64\dumpbin.exe' | Select-Object -First 1
if (-not $dumpbin) { throw 'dumpbin.exe was not found' }
$imports = & $dumpbin /DEPENDENTS $binary
if ($LASTEXITCODE -ne 0) { throw 'Could not inspect Windows dependencies' }
$imports | Write-Output
if ($imports -match '(?i)\b(?:VCRUNTIME|MSVCP|CONCRT)[A-Z0-9_]*\.dll\b') {
    throw 'The executable still depends on a separate Microsoft C/C++ runtime DLL'
}
$mt = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\mt.exe" | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $mt) { throw 'Windows SDK mt.exe was not found' }
$manifestPath = Join-Path $PSScriptRoot '..\target\release\embedded.manifest'
& $mt.FullName "-inputresource:$binary;#1" "-out:$manifestPath"
if ($LASTEXITCODE -ne 0) { throw 'The embedded manifest could not be extracted' }
[xml]$manifest = Get-Content -Raw $manifestPath
$level = $manifest.SelectSingleNode("//*[local-name()='requestedExecutionLevel']")
if (-not $level -or $level.level -ne 'requireAdministrator') {
    throw 'The release executable does not request administrator permissions'
}
Write-Output 'Windows runtime linkage and UAC manifest verified.'

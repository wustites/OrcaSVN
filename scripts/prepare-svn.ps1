param([ValidateSet('x64','arm64','ia32')][string]$Architecture = 'x64')
$ErrorActionPreference = 'Stop'
# A build launched from PowerShell 7 can otherwise hide Windows PowerShell's
# own utility/archive modules from this PowerShell 5 process.
$env:PSModulePath = "$PSHOME\Modules;${env:ProgramFiles}\WindowsPowerShell\Modules;${env:PSModulePath}"
Import-Module Microsoft.PowerShell.Utility
Import-Module Microsoft.PowerShell.Archive
$root = Split-Path -Parent $PSScriptRoot
$packages = @{
  x64 = @('x64', '77D4FE02999DDA3BDC3A20E86243AE6EDE99AAF072B4C12B0CDEDB54D88E954A')
  arm64 = @('arm64', 'A912BFF8358BBDE801E1CF4B0B32B80A977BC0B3A5E06E1BC726DE05D0E600B6')
  ia32 = @('win32', '033A4CFD484B6BB43F08D52D99C93748C40E2CB3BCBC30B63912872B6EB8CACC')
}
$package = $packages[$Architecture]
$output = Join-Path $root "src-tauri/runtime/windows-$Architecture.zip"
$receipt = "$output.json"
$generatorHash = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash
if ((Test-Path -LiteralPath $output) -and (Test-Path -LiteralPath $receipt)) {
  try { $saved = Get-Content -LiteralPath $receipt -Raw | ConvertFrom-Json } catch { $saved = $null }
  if ($saved.generator -eq $generatorHash -and $saved.upstream -eq $package[1] -and (Get-FileHash -LiteralPath $output -Algorithm SHA256).Hash -eq $saved.archive) {
    Write-Output "SVN UTF-8 runtime ready: $Architecture"
    exit 0
  }
}
$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
$hostArch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'arm64' } else { 'x64' }
$mt = Get-ChildItem -LiteralPath $sdkRoot -Directory | Sort-Object Name -Descending | ForEach-Object {
  Join-Path $_.FullName "$hostArch\mt.exe"
} | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (!$mt) { throw 'Windows SDK mt.exe is required to prepare the private UTF-8 SVN runtime.' }
$cache = Join-Path $root 'node_modules\.cache\orcasvn-svn'
New-Item -ItemType Directory -Force -Path $cache | Out-Null
$archive = Join-Path $cache "slik-$($package[0]).zip"
if (!(Test-Path -LiteralPath $archive) -or (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $package[1]) {
  Invoke-WebRequest "https://sliksvn.com/download/slik-subversion-1-14-5-$($package[0])/" -OutFile "$archive.download"
  if ((Get-FileHash -LiteralPath "$archive.download" -Algorithm SHA256).Hash -ne $package[1]) { throw 'SVN upstream archive SHA-256 mismatch.' }
  Move-Item -LiteralPath "$archive.download" -Destination $archive -Force
}
$stage = Join-Path $cache ('.prepare-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage | Out-Null
try {
  Expand-Archive -LiteralPath $archive -DestinationPath (Join-Path $stage 'archive')
  $msi = Get-ChildItem -LiteralPath (Join-Path $stage 'archive') -Filter '*.msi' | Select-Object -First 1
  if (!$msi) { throw 'SVN archive does not contain the expected MSI.' }
  $image = Join-Path $stage 'image'
  # Administrative extraction only: no installation, PATH or registry changes.
  $process = Start-Process msiexec.exe -WindowStyle Hidden -ArgumentList "/a `"$($msi.FullName)`" /qn TARGETDIR=`"$image`"" -Wait -PassThru
  if ($process.ExitCode -ne 0) { throw "SVN administrative extraction failed: $($process.ExitCode)" }
  $sourceExe = Get-ChildItem -LiteralPath $image -Recurse -Filter 'svn.exe' | Select-Object -First 1
  if (!$sourceExe) { throw 'Extracted SVN executable is missing.' }
  $sourceBin = $sourceExe.Directory.FullName
  $sourceRoot = Split-Path -Parent $sourceBin
  $runtime = Join-Path $stage 'runtime'
  $bin = Join-Path $runtime 'bin'
  New-Item -ItemType Directory -Path $bin -Force | Out-Null
  Get-ChildItem -LiteralPath $sourceBin -File | Where-Object { $_.Extension -eq '.dll' -or $_.Name -in @('svn.exe','svnadmin.exe') } | Copy-Item -Destination $bin
  $pe = [IO.File]::ReadAllBytes((Join-Path $bin 'svn.exe'))
  $machine = [BitConverter]::ToUInt16($pe, [BitConverter]::ToInt32($pe, 0x3c) + 4)
  $expectedMachine = @{ x64 = 0x8664; arm64 = 0xaa64; ia32 = 0x14c }[$Architecture]
  if ($machine -ne $expectedMachine) { throw 'SVN executable target architecture does not match the requested build.' }
  Copy-Item -LiteralPath (Join-Path $sourceRoot 'Licenses') -Destination $runtime -Recurse
  foreach ($name in @('svn.exe','svnadmin.exe')) {
    $exe = Join-Path $bin $name
    $manifestPath = Join-Path $stage "$name.manifest"
    & $mt "-inputresource:$exe;#1" "-out:$manifestPath" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Cannot read $name manifest." }
    [xml]$manifest = Get-Content -LiteralPath $manifestPath -Raw
    $ns = New-Object System.Xml.XmlNamespaceManager($manifest.NameTable)
    $ns.AddNamespace('asmv3', 'urn:schemas-microsoft-com:asm.v3')
    $application = $manifest.DocumentElement.SelectSingleNode('asmv3:application', $ns)
    if (!$application) {
      $application = $manifest.CreateElement('application', 'urn:schemas-microsoft-com:asm.v3')
      [void]$manifest.DocumentElement.AppendChild($application)
    }
    $settings = $application.SelectSingleNode('asmv3:windowsSettings', $ns)
    if (!$settings) {
      $settings = $manifest.CreateElement('windowsSettings', 'urn:schemas-microsoft-com:asm.v3')
      [void]$application.AppendChild($settings)
    }
    $codepage = $manifest.CreateElement('activeCodePage', 'http://schemas.microsoft.com/SMI/2019/WindowsSettings')
    $codepage.InnerText = 'UTF-8'
    [void]$settings.AppendChild($codepage)
    $manifest.Save($manifestPath)
    & $mt -manifest $manifestPath "-outputresource:$exe;#1" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Cannot set $name UTF-8 manifest." }
  }
  @"
OrcaSVN private UTF-8 runtime, based on SlikSVN 1.14.5 ($Architecture).
Source archive: https://sliksvn.com/download/slik-subversion-1-14-5-$($package[0])/
Upstream archive SHA-256: $($package[1])
OrcaSVN modification: svn.exe and svnadmin.exe application manifests declare
activeCodePage=UTF-8. Executable code and DLLs are unchanged. These modified
executables are not the original vendor-signed binaries. See Licenses/ for
Subversion, APR, runtime and dependency license/notice files.
Source: https://github.com/apache/subversion/tree/1.14.5
"@ | Set-Content -LiteralPath (Join-Path $runtime 'ORCASVN-NOTICE.txt') -Encoding UTF8
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $output) | Out-Null
  Add-Type -AssemblyName System.IO.Compression.FileSystem
  $stagedZip = Join-Path $stage 'runtime.zip'
  [System.IO.Compression.ZipFile]::CreateFromDirectory($runtime, $stagedZip)
  Move-Item -LiteralPath $stagedZip -Destination $output -Force
  @{ archive = (Get-FileHash -LiteralPath $output -Algorithm SHA256).Hash; generator = $generatorHash; upstream = $package[1] } |
    ConvertTo-Json | Set-Content -LiteralPath $receipt -Encoding ASCII
  Write-Output "Prepared SVN UTF-8 runtime: $output"
} finally {
  if ((Split-Path -Parent ([IO.Path]::GetFullPath($stage))) -eq ([IO.Path]::GetFullPath($cache))) {
    Remove-Item -LiteralPath $stage -Recurse -Force
  }
}

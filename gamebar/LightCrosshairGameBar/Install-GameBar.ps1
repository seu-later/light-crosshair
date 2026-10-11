$ErrorActionPreference = 'Stop'

$package = Get-ChildItem (Join-Path $PSScriptRoot 'AppPackages') -Filter '*.msix' -Recurse |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
if (!$package) { throw 'Build the Game Bar package before running this installer.' }

$certificate = Get-ChildItem $package.DirectoryName -Filter '*.cer' | Select-Object -First 1
if (!$certificate) { throw 'The package signing certificate is missing.' }

$thumbprint = ([System.Security.Cryptography.X509Certificates.X509Certificate2]::new($certificate.FullName)).Thumbprint
if (!(Test-Path "Cert:\LocalMachine\TrustedPeople\$thumbprint")) {
    Write-Host 'Approve the administrator prompt to trust the LightCrosshair development certificate.'
    $trust = Start-Process certutil.exe -Verb RunAs -Wait -PassThru -ArgumentList @(
        '-addstore', '-f', 'TrustedPeople', $certificate.FullName
    )
    if ($trust.ExitCode -ne 0) { throw "Certificate installation failed with exit code $($trust.ExitCode)." }
}

$dependencies = Get-ChildItem (Join-Path $package.DirectoryName 'Dependencies\x64') -Filter '*.appx'
Add-AppxPackage -Path $package.FullName -DependencyPath $dependencies.FullName

Write-Host 'LightCrosshair Game Bar installed successfully.' -ForegroundColor Green
Write-Host 'Open Xbox Game Bar, select LightCrosshair, and pin the widget.'

param(
    [Parameter(Mandatory)]
    [string]$PackagePath,

    [Parameter(Mandatory)]
    [string]$CertificatePath,

    [Parameter(Mandatory)]
    [string]$ExternalLocation,

    [Parameter(Mandatory)]
    [string]$PackageName
)

$ErrorActionPreference = 'Stop'

function Test-IsElevated {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Quote-Argument([string]$Value) {
    '"{0}"' -f $Value
}

$certificate = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($CertificatePath)
try {
    $trustedCertificatePath = "Cert:\LocalMachine\TrustedPeople\$($certificate.Thumbprint)"
} finally {
    $certificate.Dispose()
}

if (-not (Test-Path -LiteralPath $trustedCertificatePath)) {
    if (Test-IsElevated) {
        Import-Certificate -FilePath $CertificatePath -CertStoreLocation 'Cert:\LocalMachine\TrustedPeople' | Out-Null
    } else {
        $arguments = '-addstore TrustedPeople ' + (Quote-Argument $CertificatePath)
        $process = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\certutil.exe') -ArgumentList $arguments -Verb RunAs -WindowStyle Hidden -Wait -PassThru
        if ($process.ExitCode -ne 0) {
            throw "Publisher certificate installation failed with exit code $($process.ExitCode)."
        }
    }
    if (-not (Test-Path -LiteralPath $trustedCertificatePath)) {
        throw 'The publisher certificate was not installed.'
    }
}

Add-AppxPackage -Path $PackagePath -ExternalLocation $ExternalLocation -ForceUpdateFromAnyVersion -ForceTargetApplicationShutdown

$installed = Get-AppxPackage -Name $PackageName -ErrorAction Stop
if ($null -eq $installed -or [int]$installed.Status -ne 0) {
    throw "Windows app identity registration could not be verified: $PackageName"
}
Write-Output "Registered Windows app identity: $($installed.PackageFullName)"

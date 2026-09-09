param([Parameter(Mandatory)][string[]]$Paths)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($Paths.Count -eq 0) { throw 'No Windows signatures to verify' }
foreach ($path in $Paths) {
    $signature = Get-AuthenticodeSignature -LiteralPath $path
    if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate) {
        throw "Invalid Authenticode signature: $path ($($signature.Status))"
    }
    if ($null -eq $signature.TimeStamperCertificate) {
        throw "Missing Authenticode timestamp: $path"
    }
    Write-Output "Verified Authenticode signature and timestamp: $([IO.Path]::GetFileName($path))"
}

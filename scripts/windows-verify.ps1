param([Parameter(Mandatory)][string[]]$Paths)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($Paths.Count -eq 0) { throw 'No Windows signatures to verify' }
foreach ($path in $Paths) {
    $signature = Get-AuthenticodeSignature -LiteralPath $path
    if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate) {
        Write-Output "Windows trust result: $($signature.StatusMessage)"
        if ($null -ne $signature.SignerCertificate) {
            $chain = [Security.Cryptography.X509Certificates.X509Chain]::new()
            try {
                $chain.ChainPolicy.RevocationMode = 'Online'
                $trusted = $chain.Build($signature.SignerCertificate)
                Write-Output "Certificate chain trusted: $trusted"
                foreach ($entry in $chain.ChainElements) {
                    Write-Output "Certificate issuer: $($entry.Certificate.Issuer)"
                    foreach ($status in $entry.ChainElementStatus) {
                        Write-Output "Certificate trust error: $($status.Status) $($status.StatusInformation)"
                    }
                }
            } finally { $chain.Dispose() }
        }
        throw "Invalid Authenticode signature: $path ($($signature.Status)): $($signature.StatusMessage)"
    }
    if ($null -eq $signature.TimeStamperCertificate) {
        throw "Missing Authenticode timestamp: $path"
    }
    Write-Output "Verified Authenticode signature and timestamp: $([IO.Path]::GetFileName($path))"
}

# Analyze Unity.Licensing.Client.exe
$exePath = "C:\Program Files\Unity\Hub\Editor\6000.7.0a3\Editor\Data\Resources\Licensing\Client\Unity.Licensing.Client.exe"
$bytes = [System.IO.File]::ReadAllBytes($exePath)

Write-Host "=== PE Header Analysis ==="
Write-Host "File size: $($bytes.Length) bytes ($([math]::Round($bytes.Length/1MB, 1)) MB)"

# PE offset
$peOffset = [BitConverter]::ToInt32($bytes, 0x3C)
Write-Host "PE offset: 0x$($peOffset.ToString('X'))"

# Machine type
$machine = [BitConverter]::ToUInt16($bytes, $peOffset + 4)
Write-Host "Machine: 0x$($machine.ToString('X4'))" -NoNewline
if ($machine -eq 0x8664) { Write-Host " (x64)" } elseif ($machine -eq 0x14C) { Write-Host " (x86)" } else { Write-Host "" }

$numSections = [BitConverter]::ToUInt16($bytes, $peOffset + 6)
Write-Host "Number of sections: $numSections"

# Optional header
$optOffset = $peOffset + 24
$optMagic = [BitConverter]::ToUInt16($bytes, $optOffset)
Write-Host "Optional header magic: 0x$($optMagic.ToString('X4'))" -NoNewline
if ($optMagic -eq 0x20B) { Write-Host " (PE32+)" } elseif ($optMagic -eq 0x10B) { Write-Host " (PE32)" } else { Write-Host "" }

# Entry point
if ($optMagic -eq 0x20B) {
    $entryPoint = [BitConverter]::ToUInt32($bytes, $optOffset + 16)
    $imageBase = [BitConverter]::ToUInt64($bytes, $optOffset + 24)
} else {
    $entryPoint = [BitConverter]::ToUInt32($bytes, $optOffset + 16)
    $imageBase = [BitConverter]::ToUInt32($bytes, $optOffset + 28)
}
Write-Host "Entry point RVA: 0x$($entryPoint.ToString('X8'))"
Write-Host "Image base: 0x$($imageBase.ToString('X16'))"

# Sections
Write-Host "`n=== Sections ==="
$optHeaderSize = if ($optMagic -eq 0x20B) { 240 } else { 224 }
$sectionOffset = $optOffset + $optHeaderSize
for ($i = 0; $i -lt $numSections; $i++) {
    $sOff = $sectionOffset + ($i * 40)
    $nameBytes = $bytes[$sOff..($sOff+7)]
    $name = [System.Text.Encoding]::ASCII.GetString($nameBytes).Trim("`0")
    $vSize = [BitConverter]::ToUInt32($bytes, $sOff + 8)
    $vAddr = [BitConverter]::ToUInt32($bytes, $sOff + 12)
    $rawSize = [BitConverter]::ToUInt32($bytes, $sOff + 16)
    $rawOff = [BitConverter]::ToUInt32($bytes, $sOff + 20)
    $chars = [BitConverter]::ToUInt32($bytes, $sOff + 36)
    Write-Host ("  {0,-12} VA=0x{1:X8} VSize=0x{2:X8} Raw=0x{3:X8} RawSize=0x{4:X8} Chars=0x{5:X8}" -f $name, $vAddr, $vSize, $rawOff, $rawSize, $chars)
}

# Search for interesting strings (ASCII)
Write-Host "`n=== License-related strings (ASCII) ==="
$asciiText = [System.Text.Encoding]::ASCII.GetString($bytes)
$patterns = @("license", "License", "signature", "Signature", "validate", "Validate", "entitlement", "Entitlement", "RSA", "certificate", "Certificate", "token", "Token", "activation", "Activation", "offline", "Offline", "online", "Online", "check", "Check", "verify", "Verify", "invalid", "Invalid", "expired", "Expired", "grace", "Grace", "trial", "Trial", "serial", "Serial", "key", "Key", "hardware", "Hardware", "machine", "Machine", "fingerprint", "Fingerprint")

foreach ($pattern in $patterns) {
    $idx = 0
    $found = @()
    while (($idx = $asciiText.IndexOf($pattern, $idx)) -ne -1 -and $found.Count -lt 3) {
        $start = [Math]::Max(0, $idx - 20)
        $end = [Math]::Min($asciiText.Length, $idx + $pattern.Length + 40)
        $context = $asciiText.Substring($start, $end - $start) -replace '[^\x20-\x7E]', '.'
        $found += "  0x$($idx.ToString('X8')): ...$context..."
        $idx += $pattern.Length
    }
    if ($found.Count -gt 0) {
        Write-Host "`n[$pattern]"
        $found | ForEach-Object { Write-Host $_ }
    }
}

# Search for URLs
Write-Host "`n=== URLs ==="
$urlMatches = [regex]::Matches($asciiText, 'https?://[^\x00\x20\x22\x3C\x3E\x5C\x5E\x60\x7B\x7D\x7F]{5,200}')
foreach ($m in $urlMatches) {
    Write-Host "  0x$($m.Index.ToString('X8')): $($m.Value)"
}

# Search for interesting file extensions
Write-Host "`n=== File references ==="
$fileMatches = [regex]::Matches($asciiText, '[A-Za-z]:\\[^\x00\x20\x22\x3C\x3E]{5,200}')
foreach ($m in $fileMatches) {
    Write-Host "  0x$($m.Index.ToString('X8')): $($m.Value)"
}

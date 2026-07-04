---
name: dotnet-reverse-engineering
description: >
  Reverse-engineer .NET executables using IDA MCP, PowerShell reflection,
  ILSpy decompilation, and RSA key extraction. Use when asked to analyze
  a .NET binary, extract embedded resources, decompile obfuscated code,
  or understand crypto/license logic in a .NET assembly.
---

# .NET Binary Reverse Engineering

## Prerequisites

- IDA Pro with `ida-mcp-rs` MCP server (for native/PE analysis)
- PowerShell with `[System.Reflection.Assembly]` (built-in on Windows)
- `ilspycmd` CLI: `dotnet tool install --global ilspycmd`
- `de4dot` for deobfuscation (download latest release if needed)

## Workflow

### Step 1: Initial triage

```powershell
# Check if it's a .NET assembly
[System.Reflection.Assembly]::LoadFile("<path_to_exe>").GetTypes() | Select-Object -First 20 Name, Namespace

# List embedded resources
$asm = [System.Reflection.Assembly]::LoadFile("<path>")
$asm.GetManifestResourceNames()

# Check for obfuscation indicators
$types = $asm.GetTypes()
$types | Where-Object { $_.Name -match '^[A-Z]{1,2}$' -or $_.Name -match '^\d+$' }
```

### Step 2: Deobfuscation (if needed)

```powershell
# Use de4dot to clean obfuscated assemblies
de4dot.exe <input.exe> -o <output_cleaned.exe>

# Verify the cleaned output loads correctly
[System.Reflection.Assembly]::LoadFile("<output_cleaned.exe>").GetTypes()
```

### Step 3: ILSpy decompilation

```powershell
ilspycmd <exe_path> -o <output_dir>

# If ilspycmd fails on obfuscated code, try de4dot output first
ilspycmd <de4dot_output.exe> -o <decompiled_dir>
```

### Step 4: IDA MCP analysis (for native/PE-level details)

Use IDA MCP tools for:
- Entry points: `mcp__ida__entrypoints`
- Function analysis: `mcp__ida__analyze_funcs`
- String search: `mcp__ida__analyze_strings`
- Disassembly: `mcp__ida__disasm`

### Step 5: RSA/crypto key extraction

When dealing with license systems (ALF/ULF):

```powershell
# Search for RSA-related strings in decompiled code
Get-ChildItem "<decompiled_dir>" -Recurse -Filter "*.cs" | Select-String -Pattern "RSA|FromXml|SigningKey|PrivateKey|Modulus|Exponent"

# Extract embedded RSA keys from resources
$asm = [System.Reflection.Assembly]::LoadFile("<exe_path>")
$resourceStream = $asm.GetManifestResourceStream("<resource_name>")
$bytes = New-Object byte[] $resourceStream.Length
$resourceStream.Read($bytes, 0, $bytes.Length)
[System.IO.File]::WriteAllBytes("<output.bin>", $bytes)
```

### Step 6: Compile analysis tools

```powershell
# Use .NET Framework csc.exe for maximum compatibility
$env:PATH = "C:\Windows\Microsoft.NET\Framework64\v4.0.30319;$env:PATH"
csc.exe /out:<output.exe> <source.cs>

# Or use dotnet SDK for modern .NET targets
dotnet build
```

## Common patterns in Unity license analysis

- ALF (Activation License File): XML with machine bindings, generated locally
- ULF (Unity License File): ALF + RSA signature from Unity servers
- RSA private key: Embedded as LZMA-compressed resource in LicenseGenerate.exe
- Key extraction flow: resource → LZMA decompress → raw RSA bytes → XML format

## Gotchas

- `csc.exe` path varies by .NET version; try both Framework64 and Framework paths
- Obfuscated assemblies may have renamed methods (smethod_0, smethod_1, etc.)
- LZMA-compressed RSA keys have a specific header: `5D 00 00 80 00 00 2A 04`
- Use `--version` flags cautiously with de4dot; default detection usually works

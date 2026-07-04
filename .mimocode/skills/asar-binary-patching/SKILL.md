---
name: asar-binary-patching
description: >
  Extract, analyze, and patch ASAR archives (Electron app bundles).
  Use when modifying Unity Hub, VS Code, or other Electron apps'
  bundled JavaScript for config bypass, feature unlock, or behavior
  modification. Covers string hunting, hex offset analysis, and
  binary patching with size-matched replacements.
---

# ASAR Binary Patching

## Prerequisites

- `npx @electron/asar` (or `asar` globally)
- `grep` with binary search support (`grep -a`)
- `dd` for byte-level inspection
- PowerShell for binary file reading

## Workflow

### Step 1: Extract and analyze

```bash
# Extract ASAR
npx asar extract "<path>/app.asar" <output_dir>

# Search for target strings without extracting
grep -a "target_string" "<path>/app.asar" | head -5

# Find byte offsets of target strings
grep -boa "target_string" "<path>/app.asar"
```

### Step 2: Context analysis at offset

```bash
# Read N bytes at offset (dd with skip/count)
dd if="<path>/app.asar" bs=1 skip=<offset> count=<N> 2>/dev/null | cat -v

# Search for surrounding context
dd if="<path>/app.asar" bs=1 skip=<offset> count=1000 2>/dev/null | grep -a "context_pattern"
```

### Step 3: Binary patching (size-matched)

**CRITICAL**: Replacement must be EXACTLY the same byte length as original.

```bash
# Example: Replace a check with a bypass
# Original: "hubDisableSignInRequired === true"  (35 bytes)
# Replace:  "true||hubDisableSignInRequired===true" → WRONG (different length)
# Replace:  "true/*pad*/hubDisableSignInRequired " → Must pad to same length

# Use Edit tool with exact string match for ASAR content
```

### Step 4: Patch ASAR in-place

The UniFree project patches ASAR via Rust (`patcher.rs`):
- Uses byte-level search/replace on the raw ASAR data
- Replacement strings must match original length exactly
- Log errors if length mismatch: `"Replacement changed file size (X -> Y), not supported"`

### Step 5: Verify

```bash
# After patching, verify the change
grep -boa "replacement_string" "<path>/app.asar"

# Test the application
# For Unity Hub: launch and check logs
tail -100 "$APPDATA/UnityHub/logs/info-log.json" | grep -i "target_feature"
```

## Common Unity Hub targets

| Target | String to find | Patch approach |
|--------|---------------|----------------|
| Disable sign-in | `hubDisableSignInRequired === true` | Force `true` bypass |
| Disable auto-update | `hubDisableAutoUpdate` | Override check |
| Config endpoint | `hubConfig.json` URL | Point to local/noop |
| License validation | `enableLicenseValidation` | Set to `false` |

## Gotchas

- ASAR files are binary; `grep -a` is required for text search in binary
- Replacement length MUST match exactly or the ASAR structure breaks
- Some checks run on startup AND periodically; patch all occurrences
- `hubConfig.json` may be fetched from remote and overwrite local changes
- Use `dd` for precise offset inspection, not text editors
- The `grep -boa` output gives byte offsets from file start

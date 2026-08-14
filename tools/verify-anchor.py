#!/usr/bin/env python3
"""Verify the ValidateServerProcess anchor and patch bytes against the real Unity.exe."""
import struct
import pefile

EXE = r"D:\Unity\Editor\2019.4.40f1\Editor\Unity.exe"
pe = pefile.PE(EXE, fast_load=False)
image_base = pe.OPTIONAL_HEADER.ImageBase
data = open(EXE, "rb").read()

ANCHOR = "83 FF 09 0F 84 ?? ?? ?? ?? 49 8B CF 48 8D 15 ?? ?? ?? ?? 80 3C 11 00"

def parse(pat):
    out = []
    for b in pat.split():
        out.append(None if b == "??" else int(b, 16))
    return out

pat = parse(ANCHOR)

# search whole file
hits = []
for i in range(len(data) - len(pat)):
    ok = True
    for j, p in enumerate(pat):
        if p is not None and data[i+j] != p:
            ok = False
            break
    if ok:
        hits.append(i)

print(f"Anchor matches in whole file: {len(hits)}")
for h in hits:
    va = image_base + pe.get_rva_from_offset(h)
    print(f"  file 0x{h:X} VA 0x{va:X}")

# The patch replaces the first 3 bytes (83 FF 09) with 3B FF 90
# Verify the original bytes at the hit
if hits:
    h = hits[0]
    print(f"\nOriginal bytes at anchor: {' '.join(f'{b:02X}' for b in data[h:h+3])}")
    print(f"Patch bytes:             3B FF 90  (cmp edi,edi; nop)")

    # verify the je target (rel32 at h+4)
    je_rel = struct.unpack_from('<i', data, h+4)[0]
    je_addr = image_base + pe.get_rva_from_offset(h) + 3
    je_target = je_addr + 6 + je_rel
    print(f"je target: VA 0x{je_target:X} (should be the `mov bl, 1` success block)")

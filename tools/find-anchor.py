#!/usr/bin/env python3
"""Extract candidate anchor bytes near the ValidateServerProcess decision point
and verify uniqueness across the binary."""
import struct
import pefile

EXE = r"D:\Unity\Editor\2019.4.40f1\Editor\Unity.exe"
pe = pefile.PE(EXE, fast_load=False)
image_base = pe.OPTIONAL_HEADER.ImageBase
data = open(EXE, "rb").read()

def off(va):
    return pe.get_offset_from_rva(va - image_base)

def va_of(foff):
    return image_base + pe.get_rva_from_offset(foff)

# Decision point bytes (cmp edi,9; je; mov rcx,r15; lea rdx,rip)
base = off(0x141A55E43)
raw = data[base : base + 32]
print("Raw bytes at 0x141A55E43:")
print(' '.join(f'{b:02X}' for b in raw))

# Function entry bytes
entry = off(0x141A55C50)
eraw = data[entry : entry + 24]
print("\nFunction entry bytes at 0x141A55C50:")
print(' '.join(f'{b:02X}' for b in eraw))

# Build anchor patterns with wildcards and test uniqueness
def pattern_with_wildcards(raw, wildcard_ranges):
    """Return (byte_list, display) where byte_list has None for wildcards."""
    out = []
    disp = []
    wc = set()
    for s, e in wildcard_ranges:
        wc.update(range(s, e))
    for i, b in enumerate(raw):
        if i in wc:
            out.append(None)
            disp.append("??")
        else:
            out.append(b)
            disp.append(f"{b:02X}")
    return out, ' '.join(disp)

def count_matches(pat):
    n = len(pat)
    hits = []
    # search only in .text (code) for speed and relevance
    text = None
    for s in pe.sections:
        if s.Name.rstrip(b'\x00') == b'.text':
            text = s
    buf = data[text.PointerToRawData : text.PointerToRawData + text.SizeOfRawData]
    for i in range(len(buf) - n):
        ok = True
        for j, p in enumerate(pat):
            if p is not None and buf[i+j] != p:
                ok = False
                break
        if ok:
            hits.append(text.PointerToRawData + i)
    return hits

# Candidate 1: cmp edi,9; je (wildcard rel32); mov rcx,r15; lea rdx (wildcard disp32)
pat1, d1 = pattern_with_wildcards(raw[:18], [(5, 9), (13, 17)])
h1 = count_matches(pat1)
print(f"\nAnchor1 ({d1}) matches: {len(h1)}")
for h in h1[:5]:
    print(f"   0x{va_of(h):X}")

# Candidate 2: shorter, cmp edi,9; je rel32 only (but rel32 depends, wildcard it)
pat2, d2 = pattern_with_wildcards(raw[:9], [(5, 9)])
h2 = count_matches(pat2)
print(f"\nAnchor2 ({d2}) matches: {len(h2)}")

# Candidate 3: entry prologue + a bit more (full 24 bytes exact)
pat3, d3 = pattern_with_wildcards(eraw, [])
h3 = count_matches(pat3)
print(f"\nAnchor3 (entry 24B exact) ({d3}) matches: {len(h3)}")

# Candidate 4: entry prologue 20 bytes with sub rsp imm wildcard
pat4, d4 = pattern_with_wildcards(eraw[:20], [(16, 20)])
h4 = count_matches(pat4)
print(f"\nAnchor4 ({d4}) matches: {len(h4)}")

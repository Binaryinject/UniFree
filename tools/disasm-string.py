#!/usr/bin/env python3
"""Find xrefs to a string in Unity.exe and disassemble the referencing function."""
import struct, sys
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_64

EXE = r"D:\Unity\Editor\2019.4.40f1\Editor\Unity.exe"
pe = pefile.PE(EXE, fast_load=False)
image_base = pe.OPTIONAL_HEADER.ImageBase
data = open(EXE, "rb").read()

def off(va):
    return pe.get_offset_from_rva(va - image_base)

def find_func_start(va):
    foff = off(va)
    back = foff
    while back > 0:
        if data[back] == 0xCC:
            start = back + 1
            while start < len(data) and data[start] == 0xCC:
                start += 1
            return image_base + pe.get_rva_from_offset(start)
        back -= 1
    return image_base + pe.get_rva_from_offset(max(0, foff - 0x300))

md = Cs(CS_ARCH_X86, CS_MODE_64)

def find_string_va(s):
    idx = data.find(s.encode())
    if idx < 0:
        print(f"string not found: {s}")
        return None
    va = image_base + pe.get_rva_from_offset(idx)
    print(f"string {s!r} @ file 0x{idx:X} VA 0x{va:X}")
    return va

def find_lea_xrefs(target_va):
    text = None
    for s in pe.sections:
        if s.Name.rstrip(b'\x00') == b'.text':
            text = s
    text_raw = data[text.PointerToRawData : text.PointerToRawData + text.SizeOfRawData]
    text_base_va = image_base + text.VirtualAddress
    hits = []
    for i in range(len(text_raw) - 6):
        b0 = text_raw[i]
        b1 = text_raw[i+1]
        if not ((b0 == 0x48 or b0 == 0x4C) and b1 == 0x8D):
            continue
        modrm = text_raw[i+2]
        if (modrm & 0xC7) != 0x05:
            continue
        disp = struct.unpack_from('<i', text_raw, i+3)[0]
        insn_va = text_base_va + i
        if insn_va + 7 + disp == target_va:
            hits.append(insn_va)
    return hits

def dump_fn(va, nbytes=0x300, label=""):
    start_va = find_func_start(va)
    chunk = data[off(start_va) : off(start_va) + nbytes]
    print(f"\n===== {label} =====")
    print(f"function start VA 0x{start_va:X}")
    for insn in md.disasm(chunk, start_va):
        mark = "  <<< XREF" if insn.address in xref_marks else ""
        print(f"  0x{insn.address:X}: {insn.mnemonic:8s} {insn.op_str}{mark}")

target = sys.argv[1] if len(sys.argv) > 1 else "Unity license information is invalid"
va = find_string_va(target)
if va:
    xrefs = find_lea_xrefs(va)
    print(f"LEA xrefs: {[hex(x) for x in xrefs]}")
    xref_marks = set(xrefs)
    for x in xrefs:
        dump_fn(x, 0x400, f"function with xref @ 0x{x:X}")

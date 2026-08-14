#!/usr/bin/env python3
"""Find callers of a given VA in Unity.exe .text."""
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

def find_callers(target_va):
    text = None
    for s in pe.sections:
        if s.Name.rstrip(b'\x00') == b'.text':
            text = s
    buf = data[text.PointerToRawData : text.PointerToRawData + text.SizeOfRawData]
    base = image_base + text.VirtualAddress
    hits = []
    for i in range(len(buf) - 4):
        if buf[i] == 0xE8:
            disp = struct.unpack_from('<i', buf, i+1)[0]
            call_va = base + i
            if call_va + 5 + disp == target_va:
                hits.append(call_va)
    return hits

target = int(sys.argv[1], 16) if len(sys.argv) > 1 else 0x141A72100
callers = find_callers(target)
print(f"Callers of 0x{target:X}: {[hex(c) for c in callers]}")

for c in callers[:8]:
    start_va = find_func_start(c)
    chunk = data[off(start_va) : off(start_va) + 0x500]
    print(f"\n===== caller @ 0x{c:X} (fn start 0x{start_va:X}) =====")
    for insn in md.disasm(chunk, start_va):
        mark = "  <<< CALL" if insn.address == c else ""
        print(f"  0x{insn.address:X}: {insn.mnemonic:8s} {insn.op_str}{mark}")

#!/usr/bin/env python3
import struct, sys
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_64

EXE = r"D:\Unity\Editor\2019.4.40f1\Editor\Unity.exe"
pe = pefile.PE(EXE, fast_load=False)
image_base = pe.OPTIONAL_HEADER.ImageBase
data = open(EXE, "rb").read()

def off(va):
    return pe.get_offset_from_rva(va - image_base)

md = Cs(CS_ARCH_X86, CS_MODE_64)

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
    return image_base + pe.get_rva_from_offset(max(0, foff - 0x200))

def dump_fn(va, nbytes=0x600, label=""):
    start_va = find_func_start(va)
    foff = off(start_va)
    chunk = data[foff : foff + nbytes]
    print(f"\n===== {label} =====")
    print(f"start VA 0x{start_va:X}")
    for insn in md.disasm(chunk, start_va):
        print(f"  0x{insn.address:X}: {insn.mnemonic:8s} {insn.op_str}")

# Innermost validation (returns 9 = success)
dump_fn(0x1416aeb00, 0x500, "innermost 0x1416aeb00 (WinVerifyTrust?)")

# Also find caller of wrapper 0x141A55C50
# search for call rel32 to 0x141A55C50 in .text
text = None
for s in pe.sections:
    if s.Name.rstrip(b'\x00') == b'.text':
        text = s
text_raw = data[text.PointerToRawData : text.PointerToRawData + text.SizeOfRawData]
text_base_va = image_base + text.VirtualAddress
target = 0x141A55C50
callers = []
for i in range(len(text_raw) - 4):
    if text_raw[i] == 0xE8:  # call rel32
        disp = struct.unpack_from('<i', text_raw, i+1)[0]
        call_va = text_base_va + i
        if call_va + 5 + disp == target:
            callers.append(call_va)
print(f"\nCallers of wrapper 0x141A55C50: {[hex(c) for c in callers]}")

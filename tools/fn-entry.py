#!/usr/bin/env python3
"""Find function entry by walking back from a VA to 0xCC padding, then find callers."""
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

def find_entry(va):
    """Walk back to nearest 0xCC padding run; return entry VA."""
    foff = off(va)
    back = foff
    while back > 0:
        if data[back] == 0xCC:
            start = back + 1
            while start < len(data) and data[start] == 0xCC:
                start += 1
            return image_base + pe.get_rva_from_offset(start)
        back -= 1
    return None

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

def dump_fn(va, nbytes=0x300, label=""):
    chunk = data[off(va) : off(va) + nbytes]
    print(f"\n===== {label} @ 0x{va:X} =====")
    for insn in md.disasm(chunk, va):
        print(f"  0x{insn.address:X}: {insn.mnemonic:8s} {insn.op_str}")

va = int(sys.argv[1], 16)
entry = find_entry(va)
print(f"entry for 0x{va:X}: 0x{entry:X}" if entry else "not found")
if entry:
    dump_fn(entry, 0x500, "function")
    callers = find_callers(entry)
    print(f"\ncallers: {[hex(c) for c in callers]}")

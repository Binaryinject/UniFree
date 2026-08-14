#!/usr/bin/env python3
"""Find all RIP-relative and movabs references to a VA in Unity.exe .text."""
import struct, sys
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_64

EXE = r"D:\Unity\Editor\2019.4.40f1\Editor\Unity.exe"
pe = pefile.PE(EXE, fast_load=False)
image_base = pe.OPTIONAL_HEADER.ImageBase
data = open(EXE, "rb").read()

def off(va):
    return pe.get_offset_from_rva(va - image_base)

target_va = int(sys.argv[1], 16)

text = None
for s in pe.sections:
    if s.Name.rstrip(b'\x00') == b'.text':
        text = s
buf = data[text.PointerToRawData : text.PointerToRawData + text.SizeOfRawData]
base = image_base + text.VirtualAddress

md = Cs(CS_ARCH_X86, CS_MODE_64)
md.detail = True

hits = []
for insn in md.disasm(buf, base):
    # RIP-relative memory operand
    for op in insn.operands:
        if op.type == 3 and op.mem.base == 0 and op.mem.index == 0:
            t = insn.address + insn.size + op.mem.disp
            if t == target_va:
                hits.append(insn.address)
        # immediate
        elif op.type == 2:
            if op.imm == target_va:
                hits.append(insn.address)

print(f"xrefs to 0x{target_va:X}: {[hex(h) for h in hits]}")

for h in hits[:6]:
    start = h - 0x60
    chunk = data[off(start) : off(h) + 0x80]
    print(f"\n===== around 0x{h:X} =====")
    for insn in md.disasm(chunk, start):
        mark = "  <<<" if insn.address == h else ""
        print(f"  0x{insn.address:X}: {insn.mnemonic:8s} {insn.op_str}{mark}")

#!/usr/bin/env python3
"""Raw search for RIP-relative disp32 resolving to a target VA."""
import struct, sys
import pefile

EXE = r"D:\Unity\Editor\2019.4.40f1\Editor\Unity.exe"
pe = pefile.PE(EXE)
image_base = pe.OPTIONAL_HEADER.ImageBase
data = open(EXE, "rb").read()

target = int(sys.argv[1], 16)

text = None
for s in pe.sections:
    if s.Name.rstrip(b'\x00') == b'.text':
        text = s
buf = data[text.PointerToRawData : text.PointerToRawData + text.SizeOfRawData]
base = image_base + text.VirtualAddress

hits = []
# scan every position as a potential disp32 (last 4 bytes of an instruction)
for i in range(len(buf) - 4):
    disp = struct.unpack_from('<i', buf, i)[0]
    rip_va = base + (i + 4)  # if disp32 is at buf[i], RIP is at buf[i+4]
    if rip_va + disp == target:
        insn_start_va = base + i - 3  # assume 7-byte LEA/MOV
        hits.append(insn_start_va)

print(f"raw RIP-relative refs to 0x{target:X}: {[hex(h) for h in hits]}")
for h in hits:
    off = pe.get_offset_from_rva(h - image_base)
    print(f"  VA 0x{h:X} file 0x{off:X}")

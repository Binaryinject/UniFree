#!/usr/bin/env python3
import struct
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_64

EXE = r"D:\Unity\Editor\2019.4.40f1\Editor\Unity.exe"

def main():
    pe = pefile.PE(EXE, fast_load=False)
    image_base = pe.OPTIONAL_HEADER.ImageBase
    data = open(EXE, "rb").read()

    text = None
    for s in pe.sections:
        if s.Name.rstrip(b'\x00') == b'.text':
            text = s

    needle = b"Error {0} while verifying Licensing Client signature"
    moff = data.find(needle)
    m_va = image_base + pe.get_rva_from_offset(moff)
    print(f"Error string VA = 0x{m_va:X}")

    text_raw = data[text.PointerToRawData : text.PointerToRawData + text.SizeOfRawData]
    text_base_va = image_base + text.VirtualAddress

    # Raw scan for 7-byte LEA (48 8D /r disp32 or 4C 8D /r disp32) rip-relative to m_va
    hits = []
    for i in range(len(text_raw) - 6):
        b0 = text_raw[i]
        b1 = text_raw[i+1]
        if not ((b0 == 0x48 or b0 == 0x4C) and b1 == 0x8D):
            continue
        modrm = text_raw[i+2]
        if (modrm & 0xC7) != 0x05:  # mod=00, rm=101 (rip)
            continue
        disp = struct.unpack_from('<i', text_raw, i+3)[0]
        insn_va = text_base_va + i
        target = insn_va + 7 + disp
        if target == m_va:
            hits.append(insn_va)
    print(f"Found {len(hits)} LEA xrefs to error string")

    md = Cs(CS_ARCH_X86, CS_MODE_64)

    for hva in hits:
        foff = pe.get_offset_from_rva(hva - image_base)
        print(f"\n=== xref LEA @ VA 0x{hva:X} (file 0x{foff:X}) ===")
        # dump bytes
        raw = data[foff-0x60 : foff+0x180]
        base_va = hva - 0x60
        for insn in md.disasm(raw, base_va):
            mark = "  <<<" if insn.address == hva else ""
            print(f"  0x{insn.address:X}: {insn.mnemonic:8s} {insn.op_str}{mark}")

if __name__ == "__main__":
    main()

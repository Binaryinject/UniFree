import sys
import shutil
import dnfile

def patch_checksignature(dll_path):
    """Patch SignedXml.CheckSignature to always return true."""
    pe = dnfile.dnPE(dll_path)
    if not pe.net:
        return "Not a .NET assembly"

    # Find CheckSignature methods
    mdtables = pe.net.mdtables
    if not mdtables or not hasattr(mdtables, 'MethodDef'):
        return "MethodDef table not found"

    target_rows = []
    for i, method in enumerate(mdtables.MethodDef):
        name = str(method.Name) if method.Name else ""
        if name == "CheckSignature":
            target_rows.append((i, method))

    if not target_rows:
        return "CheckSignature method not found"

    # Check if we need to backup
    bak_path = dll_path + ".bak"
    import os
    if not os.path.exists(bak_path):
        shutil.copy2(dll_path, bak_path)

    patched = 0
    for idx, method in target_rows:
        rva = method.Rva
        if rva == 0:
            continue

        # Get file offset from RVA
        file_offset = pe.get_offset_from_rva(rva)
        if file_offset is None:
            continue

        raw = pe.__data__
        if file_offset >= len(raw):
            continue

        header = raw[file_offset]
        is_tiny = (header & 0x03) == 0x02

        if is_tiny:
            il_size = header >> 2
            il_start = file_offset + 1
        else:
            if file_offset + 12 > len(raw):
                continue
            il_size = int.from_bytes(raw[file_offset + 4:file_offset + 8], 'little')
            il_start = file_offset + 12

        if il_start + il_size > len(raw):
            continue

        il_bytes = bytes(raw[il_start:il_start + il_size])

        # Already patched?
        if il_bytes == b'\x17\x2a':
            continue

        # Patch: ldc.i4.1 (0x17) + ret (0x2A)
        raw[il_start] = 0x17
        raw[il_start + 1] = 0x2A
        # NOP fill rest
        for j in range(2, il_size):
            raw[il_start + j] = 0x00

        patched += 1

    if patched == 0:
        return "Already patched"

    # Write patched file
    with open(dll_path, 'wb') as f:
        f.write(bytes(raw))

    return f"Patched {patched} CheckSignature method(s)"

def get_status(dll_path):
    """Check if CheckSignature is already patched."""
    import os
    if not os.path.exists(dll_path):
        return "not_found"

    try:
        pe = dnfile.dnPE(dll_path)
        if not pe.net:
            return "unknown"
        mdtables = pe.net.mdtables
        if not mdtables or not hasattr(mdtables, 'MethodDef'):
            return "unknown"

        for method in mdtables.MethodDef:
            name = str(method.Name) if method.Name else ""
            if name != "CheckSignature":
                continue
            rva = method.Rva
            if rva == 0:
                continue
            file_offset = pe.get_offset_from_rva(rva)
            if file_offset is None:
                continue
            raw = pe.__data__
            header = raw[file_offset]
            is_tiny = (header & 0x03) == 0x02
            if is_tiny:
                il_size = header >> 2
                il_start = file_offset + 1
            else:
                il_size = int.from_bytes(raw[file_offset + 4:file_offset + 8], 'little')
                il_start = file_offset + 12
            if il_start + il_size > len(raw):
                continue
            il_bytes = bytes(raw[il_start:il_start + il_size])
            if il_bytes == b'\x17\x2a':
                return "patched"
            return "original"
        return "unknown"
    except Exception:
        return "error"

def restore(dll_path):
    """Restore DLL from backup."""
    import os
    bak_path = dll_path + ".bak"
    if not os.path.exists(bak_path):
        return "Backup not found"
    shutil.copy2(bak_path, dll_path)
    os.remove(bak_path)
    return f"Restored: {dll_path}"

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: il_patch.py <patch|status|restore> <dll_path>")
        sys.exit(1)

    action = sys.argv[1]
    dll_path = sys.argv[2]

    if action == "patch":
        print(patch_checksignature(dll_path))
    elif action == "status":
        print(get_status(dll_path))
    elif action == "restore":
        print(restore(dll_path))
    else:
        print(f"Unknown action: {action}")
        sys.exit(1)

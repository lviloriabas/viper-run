#!/usr/bin/env python3
"""Check capture compatibility in the final Windows PE, using only stdlib.

Inspect the final executable to ensure external performance overlays are not disabled.
"""

from pathlib import Path
import struct
import sys


def check(path: Path) -> None:
    data = path.read_bytes()

    def u16(at):
        return struct.unpack_from("<H", data, at)[0]

    def u32(at):
        return struct.unpack_from("<I", data, at)[0]

    pe = u32(0x3C)
    assert data[:2] == b"MZ" and data[pe:pe + 4] == b"PE\0\0", "not a PE file"
    assert u16(pe + 4) == 0x8664, "expected Windows x86_64"
    optional = pe + 24
    assert u16(optional) == 0x20B, "expected PE32+"
    sections = optional + u16(pe + 20)

    def offset(rva):
        for section in range(sections, sections + u16(pe + 6) * 40, 40):
            address, size, raw = u32(section + 12), u32(section + 16), u32(section + 20)
            if address <= rva < address + size:
                return raw + rva - address
        raise AssertionError(f"RVA {rva:#x} has no file-backed section")

    export_rva, export_size = u32(optional + 112), u32(optional + 116)
    if not export_rva or not export_size:
        print(f"PASS: {path.name} allows external capture and overlay hooks")
        return
    exports = offset(export_rva)
    functions, names, ordinals = (offset(u32(exports + n)) for n in (28, 32, 36))
    for index in range(u32(exports + 24)):
        name = offset(u32(names + index * 4))
        if data[name:data.index(b"\0", name)] != b"RTSSHooksCompatibility":
            continue
        ordinal = u16(ordinals + index * 2)
        assert ordinal < u32(exports + 20), "invalid export ordinal"
        marker = u32(functions + ordinal * 4)
        assert not export_rva <= marker < export_rva + export_size, "marker is a forwarder"
        assert u32(offset(marker)) != 0, "RTSSHooksCompatibility = 0 disables external overlays"
        print(f"PASS: {path.name} does not disable RTSS overlay hooks")
        return
    print(f"PASS: {path.name} allows external capture and overlay hooks")


if __name__ == "__main__":
    target = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[1] / "dist/viper-run-windows-x86_64.exe"
    check(target)

#!/usr/bin/env python3
"""Align ELF PT_LOAD segments to 16 KiB so Android 15+ 16 KB page devices can dlopen them."""

from __future__ import annotations

import os
import struct
import sys

PAGE = 0x4000
PT_LOAD = 1


def align_up(n: int, a: int) -> int:
    return (n + a - 1) & ~(a - 1)


def load(path: str) -> bytearray | None:
    with open(path, "rb") as f:
        data = bytearray(f.read())
    if data[:4] != b"\x7fELF" or data[4] != 2 or data[5] != 1:
        return None
    return data


def phdrs(data: bytearray) -> list[tuple[int, int, int, int, int]]:
    e_phoff = struct.unpack_from("<Q", data, 32)[0]
    e_phentsize = struct.unpack_from("<H", data, 54)[0]
    e_phnum = struct.unpack_from("<H", data, 56)[0]
    out = []
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_type = struct.unpack_from("<I", data, off)[0]
        p_offset = struct.unpack_from("<Q", data, off + 8)[0]
        p_vaddr = struct.unpack_from("<Q", data, off + 16)[0]
        p_align = struct.unpack_from("<Q", data, off + 48)[0]
        out.append((off, p_type, p_offset, p_vaddr, p_align))
    return out


def bump_sh_offsets(data: bytearray, insert_at: int, pad: int) -> None:
    e_shoff = struct.unpack_from("<Q", data, 40)[0]
    e_shentsize = struct.unpack_from("<H", data, 58)[0]
    e_shnum = struct.unpack_from("<H", data, 60)[0]
    if e_shoff == 0 or e_shnum == 0:
        return
    if e_shoff >= insert_at:
        struct.pack_into("<Q", data, 40, e_shoff + pad)
        e_shoff = e_shoff + pad
    for i in range(e_shnum):
        sh = e_shoff + i * e_shentsize
        if sh + 32 > len(data):
            return
        sh_offset = struct.unpack_from("<Q", data, sh + 24)[0]
        if sh_offset >= insert_at:
            struct.pack_into("<Q", data, sh + 24, sh_offset + pad)


def insert_pad(data: bytearray, insert_at: int, pad: int) -> bytearray:
    e_phoff = struct.unpack_from("<Q", data, 32)[0]
    e_phentsize = struct.unpack_from("<H", data, 54)[0]
    e_phnum = struct.unpack_from("<H", data, 56)[0]
    out = bytearray(data[:insert_at] + (b"\x00" * pad) + data[insert_at:])
    if e_phoff >= insert_at:
        e_phoff += pad
        struct.pack_into("<Q", out, 32, e_phoff)
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_offset = struct.unpack_from("<Q", out, off + 8)[0]
        if p_offset >= insert_at:
            struct.pack_into("<Q", out, off + 8, p_offset + pad)
    bump_sh_offsets(out, insert_at, pad)
    return out


def align_file(path: str) -> str:
    data = load(path)
    if data is None:
        return "skip"
    changed = False
    # Repeat: bump align, insert pad if congruence fails.
    for _ in range(16):
        headers = phdrs(data)
        need_pad = None
        for off, p_type, p_offset, p_vaddr, p_align in headers:
            if p_type != PT_LOAD:
                continue
            if p_align < PAGE:
                struct.pack_into("<Q", data, off + 48, PAGE)
                p_align = PAGE
                changed = True
            if (p_offset % PAGE) != (p_vaddr % PAGE):
                want = p_vaddr % PAGE
                have = p_offset % PAGE
                pad = (want - have) % PAGE
                if pad == 0:
                    continue
                need_pad = (p_offset, pad)
                break
        if need_pad is None:
            break
        insert_at, pad = need_pad
        data = insert_pad(data, insert_at, pad)
        changed = True
    else:
        return "fail"
    if not changed:
        return "ok"
    tmp = path + ".16k"
    with open(tmp, "wb") as f:
        f.write(data)
    os.replace(tmp, path)
    return "aligned"


def walk(root: str) -> int:
    n = 0
    if os.path.isfile(root):
        paths = [root]
    else:
        paths = []
        for dirpath, _, files in os.walk(root):
            for name in files:
                if name.endswith(".so"):
                    paths.append(os.path.join(dirpath, name))
    for path in paths:
        result = align_file(path)
        if result in ("aligned", "ok"):
            print(f"  16k {result}: {path}")
            n += 1
        elif result == "fail":
            print(f"  16k FAILED: {path}", file=sys.stderr)
        else:
            print(f"  16k skip: {path}")
    return n


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("usage: align_elf_16k.py <file-or-dir> [...]", file=sys.stderr)
        sys.exit(2)
    total = 0
    for arg in sys.argv[1:]:
        total += walk(arg)
    sys.exit(0 if total or True else 1)

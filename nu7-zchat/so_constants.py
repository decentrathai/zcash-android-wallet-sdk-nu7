#!/usr/bin/env python3
"""so_constants.py <libzcashwalletsdk.so>... — does a built native library carry NU7's consensus constants?

Reads two facts from the bytes of each library (stripped or not; arm64-v8a, armeabi-v7a, x86, x86_64):

  * the consensus branch-id table continues after NU6.3 (0x37a5165b) with NU7 (0x77190ad9): zcash_protocol's
    `u32::from(BranchId)` compiles to a table of the ids in upgrade order;
  * the testnet NU7 activation height, 4,465,026 (ZIP 259). `activation_height` is inlined into
    `height >= activation` tests, so the compiler stores the height or the height minus one: as table data or
    an x86 immediate (four little-endian bytes), as an AArch64 MOVZ/MOVK pair, or as a Thumb-2 MOVW/MOVT pair.

The libraries of SDK 3.0.0 have neither (0 hits for both in all four ABIs), so a hit is not an accident of the
surrounding data. Mainnet has no NU7 height in this build; when one is set, add it to HEIGHTS.

Prints one line per fact and library; exit 1 if a fact is missing.
"""
import struct
import sys

NU6_3_BRANCH_ID = 0x37A5165B
NU7_BRANCH_ID = 0x77190AD9
HEIGHTS = {"testnet NU7 height 4465026": 4_465_026, "mainnet NU7 height 3543405": 3_543_405}
MACHINES = {183: "aarch64", 40: "arm", 3: "x86", 62: "x86_64"}
WINDOW = 3  # the second instruction of a pair follows within this many instructions


def u32(value):
    return struct.pack("<I", value)


def aarch64_pairs(data, value):
    """MOVZ Rd, #low16 followed by MOVK Rd, #high16, LSL #16 (32-bit and 64-bit register forms)."""
    low, high = value & 0xFFFF, value >> 16
    found = 0
    for movz, movk in ((0x52800000, 0x72A00000), (0xD2800000, 0xF2A00000)):
        for rd in range(31):
            first, second = u32(movz | low << 5 | rd), u32(movk | high << 5 | rd)
            at = data.find(first)
            while at >= 0:
                if at % 4 == 0 and second in (data[at + 4 * n:at + 4 * n + 4] for n in range(1, WINDOW + 1)):
                    found += 1
                at = data.find(first, at + 1)
    return found


def thumb2_pairs(data, value):
    """MOVW Rd, #low16 followed by MOVT Rd, #high16 (Thumb-2 encodings T3 and T1)."""
    def encode(op, imm16, rd):
        imm4, i, imm3, imm8 = imm16 >> 12, imm16 >> 11 & 1, imm16 >> 8 & 7, imm16 & 0xFF
        return struct.pack("<HH", 0xF000 | i << 10 | op | imm4, imm3 << 12 | rd << 8 | imm8)

    found = 0
    for rd in range(15):
        first, second = encode(0x0240, value & 0xFFFF, rd), encode(0x02C0, value >> 16, rd)
        at = data.find(first)
        while at >= 0:
            if at % 2 == 0 and second in (data[at + 4 * n:at + 4 * n + 4] for n in range(1, WINDOW + 1)):
                found += 1
            at = data.find(first, at + 1)
    return found


def height_sites(data, machine, height):
    """Places that hold `height` or `height - 1`: (little-endian data or x86 immediates, instruction pairs)."""
    plain = sum(data.count(u32(v)) for v in (height, height - 1))
    pairs = 0
    if machine == "aarch64":
        pairs = sum(aarch64_pairs(data, v) for v in (height, height - 1))
    elif machine == "arm":
        pairs = sum(thumb2_pairs(data, v) for v in (height, height - 1))
    return plain, pairs


def main(paths):
    if not paths:
        sys.exit(__doc__.strip().splitlines()[0])
    failed = False
    for path in paths:
        with open(path, "rb") as f:
            data = f.read()
        if data[:4] != b"\x7fELF":
            sys.exit(f"{path}: not an ELF file")
        machine = MACHINES.get(struct.unpack_from("<H", data, 18)[0], "unknown")
        tables = data.count(u32(NU6_3_BRANCH_ID) + u32(NU7_BRANCH_ID))
        print(f"{path}: {machine}; branch-id tables ...37a5165b,77190ad9 (NU6.3, NU7): {tables}")
        failed |= tables == 0
        for name, height in HEIGHTS.items():
            plain, pairs = height_sites(data, machine, height)
            print(f"{path}: {name}: {plain} as data or immediate bytes, {pairs} as instruction pairs")
            failed |= plain + pairs == 0
    if failed:
        print("FAIL: an NU7 constant is missing")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

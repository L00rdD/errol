"""An EDID for a 5" 800 x 480 HDMI screen (an MPI5008), for when it sends none.

That screen only answers EDID reads in some conditions: switched off and on
again, it leaves the Pi without EDID, and the kernel then offers generic modes
the screen refuses ("no signal") until a reboot. Loading an EDID from a file at
boot makes the display independent of that (deploy/install.sh, EDID=1).

The timings are the screen's own preferred mode, read from its EDID:

    800 x 480 at 59.98 Hz, 29.57 MHz
    horizontal 800 816 896 992 (front 16, sync 80, back 96), sync -
    vertical   480 481 484 497 (front 1, sync 3, back 13),   sync +

This script builds an EDID with that single mode (base block only, no serial
number):

    python3 make_edid.py > errol-800x480.bin

For another screen, read its EDID once while it works
(`cat /sys/class/drm/card1-HDMI-A-1/edid | edid-decode`) and change the
timings below.
"""
import sys

CLOCK_10KHZ = 2957
H_ACTIVE, H_FRONT, H_SYNC, H_TOTAL = 800, 16, 80, 992
V_ACTIVE, V_FRONT, V_SYNC, V_TOTAL = 480, 1, 3, 497


def edid():
    b = bytearray(128)
    b[0:8] = bytes([0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00])
    b[8:10] = bytes([0x36, 0x09])            # manufacturer "MPI", as the screen says
    b[10:12] = bytes([0x08, 0x50])           # product 20488, as the screen says
    b[16], b[17] = 1, 2026 - 1990            # week, year
    b[18], b[19] = 1, 3                      # EDID 1.3
    b[20] = 0x80                             # digital input
    b[21], b[22] = 11, 7                     # 11 x 7 cm
    b[23] = 0x78                             # gamma 2.2
    b[24] = 0x0A                             # RGB, first detailed timing is the preferred mode
    b[25:35] = bytes([0xEE, 0x91, 0xA3, 0x54, 0x4C, 0x99, 0x26, 0x0F, 0x50, 0x54])
    b[38:54] = bytes([0x01] * 16)            # no standard timings
    h_blank, v_blank = H_TOTAL - H_ACTIVE, V_TOTAL - V_ACTIVE
    b[54:72] = bytes([
        CLOCK_10KHZ & 0xFF, CLOCK_10KHZ >> 8,
        H_ACTIVE & 0xFF, h_blank & 0xFF, (H_ACTIVE >> 8) << 4 | (h_blank >> 8),
        V_ACTIVE & 0xFF, v_blank & 0xFF, (V_ACTIVE >> 8) << 4 | (v_blank >> 8),
        H_FRONT, H_SYNC, (V_FRONT << 4) | V_SYNC, 0,
        108, 65, 0,                          # 108 x 65 mm
        0, 0,
        0x1C,                                # digital separate sync, +vsync, -hsync
    ])
    b[72:90] = bytes([0, 0, 0, 0xFC, 0]) + b"MPI5008\n".ljust(13, b" ")
    b[90:108] = bytes([0, 0, 0, 0xFD, 0, 50, 76, 30, 81, 3, 0x0A]) + b" " * 7
    b[108:126] = bytes([0, 0, 0, 0x10, 0] + [0] * 13)
    b[126] = 0                               # no extension
    b[127] = (-sum(b[:127])) & 0xFF
    return bytes(b)


if __name__ == "__main__":
    sys.stdout.buffer.write(edid())

#!/usr/bin/env python3
"""Render `tmux capture-pane -e -p` output (truecolor SGR) to a PNG.

Usage: ansi2png.py in.ansi out.png   (needs Pillow; 10x20 px cells, so 120x40 -> 1200x800;
at least 120 columns, wider when the capture is wider)
"""
import re
import sys

from PIL import Image, ImageDraw, ImageFont

CW, CH = 10, 20
FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
BG = (0x10, 0x12, 0x18)
FG = (0xD0, 0xD0, 0xD0)


def main(src, dst):
    lines = open(src, encoding="utf-8").read().split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    sgr = re.compile(r"\x1b\[([0-9;]*)m")
    cols = max([120] + [len(sgr.sub("", l)) for l in lines])
    img = Image.new("RGB", (cols * CW, len(lines) * CH), BG)
    d = ImageDraw.Draw(img)
    font = ImageFont.truetype(FONT, 16)
    for y, line in enumerate(lines):
        fg, bg, x, pos = FG, None, 0, 0
        while pos < len(line):
            m = sgr.match(line, pos)
            if m:
                p = [int(v) if v else 0 for v in m.group(1).split(";")]
                i = 0
                while i < len(p):
                    if p[i] == 0:
                        fg, bg = FG, None
                    elif p[i] in (38, 48) and i + 4 < len(p) and p[i + 1] == 2:
                        c = tuple(p[i + 2 : i + 5])
                        if p[i] == 38:
                            fg = c
                        else:
                            bg = c
                        i += 4
                    i += 1
                pos = m.end()
                continue
            ch = line[pos]
            pos += 1
            if ch == "\x1b":
                continue
            if bg:
                d.rectangle([x * CW, y * CH, (x + 1) * CW - 1, (y + 1) * CH - 1], fill=bg)
            if ch != " ":
                d.text((x * CW, y * CH), ch, font=font, fill=fg)
            x += 1
    img.save(dst)


main(sys.argv[1], sys.argv[2])

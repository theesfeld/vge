#!/usr/bin/env python3
"""
CMFD Board A/B — headless KiCad build (pcbnew).

Board A = PASSIVE SWITCH PANEL
  Each button has its own 1-pin header a few mm toward the glass.
  Tracks are short and never cross. Cable = many single dupont wires
  (or a custom harness) from those pins + GND to the MCU.

Board B = carrier (SoM, USB-C, ports) + matching bezel pin row.

You do NOT need to open KiCad. Upload the gerber zips.

Run: bash hardware/tools/kicad_export.sh
"""
from __future__ import annotations

import os
import sys
from pathlib import Path


def _find_fp_root() -> Path:
    env = os.environ.get("KICAD_FOOTPRINT_DIR")
    if env and Path(env).is_dir():
        return Path(env)
    matches = sorted(Path("/nix/store").glob("*-kicad-footprints-*/share/kicad/footprints"))
    if not matches:
        raise SystemExit("nix-shell -p kicad  # footprints missing")
    return matches[-1]


def _ensure_pcbnew():
    if "pcbnew" in sys.modules:
        return
    for c in sorted(Path("/nix/store").glob("*-kicad-base-*/lib/python*/site-packages")):
        if (c / "_pcbnew.so").exists() or (c / "pcbnew.py").exists():
            sys.path.insert(0, str(c))
            break
    import pcbnew  # noqa: F401


_ensure_pcbnew()
import pcbnew  # type: ignore  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
FP_ROOT = _find_fp_root()


def mm(v):
    return pcbnew.FromMM(v)


def to_mm(v):
    return pcbnew.ToMM(v)


def vec(x, y):
    return pcbnew.VECTOR2I(mm(x), mm(y))


def load_fp(lib, name):
    fp = pcbnew.FootprintLoad(str(FP_ROOT / f"{lib}.pretty"), name)
    if fp is None:
        raise RuntimeError(f"Missing {lib}:{name}")
    return fp


def place(board, lib, name, ref, value, x, y, rot=0.0):
    fp = load_fp(lib, name)
    fp.SetReference(ref)
    fp.SetValue(value)
    fp.SetPosition(vec(x, y))
    if rot:
        fp.SetOrientation(pcbnew.EDA_ANGLE(rot, pcbnew.DEGREES_T))
    board.Add(fp)
    return fp


def edge_rect(board, x0, y0, w, h):
    pts = [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h), (x0, y0)]
    for (a, b), (c, d) in zip(pts, pts[1:]):
        s = pcbnew.PCB_SHAPE(board)
        s.SetShape(pcbnew.SHAPE_T_SEGMENT)
        s.SetLayer(pcbnew.Edge_Cuts)
        s.SetStart(vec(a, b))
        s.SetEnd(vec(c, d))
        s.SetWidth(mm(0.1))
        board.Add(s)


def silk(board, x, y, text, size=1.0):
    t = pcbnew.PCB_TEXT(board)
    t.SetText(text)
    t.SetPosition(vec(x, y))
    t.SetLayer(pcbnew.F_SilkS)
    t.SetTextSize(pcbnew.VECTOR2I(mm(size), mm(size)))
    t.SetTextThickness(mm(max(0.12, size * 0.12)))
    board.Add(t)


def ensure_net(board, name):
    ni = board.FindNet(name)
    if ni is None:
        ni = pcbnew.NETINFO_ITEM(board, name)
        board.Add(ni)
    return ni.GetNetCode()


def pad_xy(pad):
    p = pad.GetPosition()
    return to_mm(p.x), to_mm(p.y)


def pads_num(fp, num):
    return [p for p in fp.Pads() if p.GetNumber() == num]


def first_pad(fp, num):
    ps = pads_num(fp, num)
    return ps[0] if ps else None


def track(board, x1, y1, x2, y2, netcode, width=0.3):
    if abs(x1 - x2) < 1e-4 and abs(y1 - y2) < 1e-4:
        return
    t = pcbnew.PCB_TRACK(board)
    t.SetStart(vec(x1, y1))
    t.SetEnd(vec(x2, y2))
    t.SetWidth(mm(width))
    t.SetLayer(pcbnew.F_Cu)
    t.SetNetCode(netcode)
    board.Add(t)


def set_net(fp, num, board, net):
    code = ensure_net(board, net)
    for p in pads_num(fp, num):
        p.SetNetCode(code)
    return code


def short_dual(board, fp, num, net):
    code = set_net(fp, num, board, net)
    ps = pads_num(fp, num)
    if len(ps) >= 2:
        x1, y1 = pad_xy(ps[0])
        x2, y2 = pad_xy(ps[1])
        track(board, x1, y1, x2, y2, code, 0.25)
        return (x1 + x2) / 2, (y1 + y2) / 2, code
    if ps:
        return (*pad_xy(ps[0]), code)
    return None


def direct(board, x1, y1, x2, y2, netcode, width=0.3):
    """Straight track only (no L)."""
    track(board, x1, y1, x2, y2, netcode, width)


def new_board():
    board = pcbnew.BOARD()
    ds = board.GetDesignSettings()
    ds.SetBoardThickness(mm(1.6))
    ds.m_TrackMinWidth = mm(0.2)
    ds.m_ViasMinSize = mm(0.6)
    ds.m_ViasMinDrill = mm(0.3)
    ds.m_CopperEdgeClearance = mm(0.5)
    ds.m_HoleClearance = mm(0.3)
    ds.m_HoleToHoleMin = mm(0.5)
    return board


def osb_layout(W, H, inset=14.0, span=78.0):
    step = span / 4
    c = W / 2
    out = []
    for i in range(5):
        out.append((1 + i, c - span / 2 + i * step, H - inset, "TOP", i))
    for i in range(5):
        out.append((6 + i, W - inset, H / 2 + span / 2 - i * step, "RIGHT", i))
    for i in range(5):
        out.append((15 - i, c - span / 2 + i * step, inset, "BOT", i))
    for i in range(5):
        out.append((20 - i, inset, H / 2 + span / 2 - i * step, "LEFT", i))
    return out


def pin_offset(side, x, y, dist=5.5):
    """Header toward outer edge, but keep ≥3.5 mm copper-edge clearance."""
    if side == "TOP":
        return x, y + dist
    if side == "BOT":
        return x, y - dist
    if side == "LEFT":
        return x - dist, y
    return x + dist, y  # RIGHT


def build_board_a(path: Path):
    W, H = 148.0, 148.0
    cut = 102.0
    cx0 = (W - cut) / 2
    cy0 = (H - cut) / 2

    board = new_board()
    gnd = ensure_net(board, "GND")

    edge_rect(board, 0, 0, W, H)
    edge_rect(board, cx0, cy0, cut, cut)

    silk(board, 22, H - 5, "CMFD BEZEL A", 1.0)
    silk(board, 22, H - 8, "1 pin per button", 0.7)

    for i, (x, y) in enumerate([(16, 16), (W - 16, 16), (W - 16, H - 16), (16, H - 16)], 1):
        place(board, "MountingHole", "MountingHole_3.2mm_M3", f"H{i}", "M3", x, y)

    # One GND pin near bottom-center of frame (inside frame, outside glass)
    jgnd = place(
        board,
        "Connector_PinHeader_2.54mm",
        "PinHeader_1x02_P2.54mm_Vertical",
        "J_GND",
        "GND",
        W / 2,
        20.0,
    )
    for pin in ("1", "2"):
        p = first_pad(jgnd, pin)
        if p:
            p.SetNetCode(gnd)

    lines = [
        "# Bezel wiring (Board A)",
        "",
        "Each line is one Dupont wire (or harness).",
        "",
        "| Button | Pin on board | Connect to MCU |",
        "|--------|--------------|----------------|",
    ]

    sw = {}
    for oid, x, y, side, idx in osb_layout(W, H):
        rot = {"TOP": 180.0, "BOT": 0.0, "LEFT": 90.0, "RIGHT": 270.0}[side]
        sfp = place(
            board,
            "Button_Switch_THT",
            "SW_PUSH_6mm_H4.3mm",
            f"S{oid}",
            f"OSB{oid}",
            x,
            y,
            rot,
        )
        hx, hy = pin_offset(side, x, y, 5.5)
        hx = min(max(hx, 3.5), W - 3.5)
        hy = min(max(hy, 3.5), H - 3.5)

        jfp = place(
            board,
            "Connector_PinHeader_2.54mm",
            "PinHeader_1x01_P2.54mm_Vertical",
            f"J{oid}",
            f"OSB{oid}",
            hx,
            hy,
        )
        net = f"OSB{oid}"
        code = ensure_net(board, net)
        short_dual(board, sfp, "1", net)  # bridge dual pad1s
        short_dual(board, sfp, "2", "GND")
        jp = first_pad(jfp, "1")
        if jp:
            jp.SetNetCode(code)
            jx, jy = pad_xy(jp)
            # start from the pad1 closest to the header (avoids crossing pad2/GND)
            p1s = pads_num(sfp, "1")
            if p1s:
                best = min(
                    p1s,
                    key=lambda p: (pad_xy(p)[0] - jx) ** 2 + (pad_xy(p)[1] - jy) ** 2,
                )
                sx, sy = pad_xy(best)
                direct(board, sx, sy, jx, jy, code, 0.3)
        sw[oid] = (sfp, side)
        lines.append(f"| OSB {oid} | **J{oid}** | any free GPIO (see firmware map) |")

    # Rockers with local pins
    rk = {
        "GAIN": (26.0, H - 26.0),
        "SYM": (W - 26.0, H - 26.0),
        "BRT": (26.0, 26.0),
        "CON": (W - 26.0, 26.0),
    }
    for i, (name, (x, y)) in enumerate(rk.items(), 1):
        sfp = place(board, "Button_Switch_THT", "SW_PUSH_6mm_H4.3mm", f"SW{i}", name, x, y)
        # pin toward outer corner (away from glass / GND pads)
        hx = x + (-5.5 if x < W / 2 else 5.5)
        hy = y + (-5.5 if y < H / 2 else 5.5)
        hx = min(max(hx, 3.5), W - 3.5)
        hy = min(max(hy, 3.5), H - 3.5)

        jfp = place(
            board,
            "Connector_PinHeader_2.54mm",
            "PinHeader_1x01_P2.54mm_Vertical",
            f"J_{name}",
            name,
            hx,
            hy,
        )
        net = f"RK_{name}"
        code = ensure_net(board, net)
        short_dual(board, sfp, "1", net)
        short_dual(board, sfp, "2", "GND")
        jp = first_pad(jfp, "1")
        if jp:
            jp.SetNetCode(code)
            jx, jy = pad_xy(jp)
            p1s = pads_num(sfp, "1")
            if p1s:
                best = min(
                    p1s,
                    key=lambda p: (pad_xy(p)[0] - jx) ** 2 + (pad_xy(p)[1] - jy) ** 2,
                )
                direct(board, *pad_xy(best), jx, jy, code, 0.3)
        silk(board, x - 4, y + 7.5, name, 0.7)
        lines.append(f"| {name} | **J_{name}** | GPIO |")

        # GND hop to nearest OSB pad2 (short)
        near = {"GAIN": 1, "SYM": 5, "BRT": 15, "CON": 11}[name]
        if near in sw:
            set_net_ok = set_net(sfp, "2", board, "GND")
            nfp = sw[near][0]
            set_net(nfp, "2", board, "GND")
            pa, pb = first_pad(sfp, "2"), first_pad(nfp, "2")
            if pa and pb:
                direct(board, *pad_xy(pa), *pad_xy(pb), gnd, 0.35)

    # GND chains along each side (pad2 only) — adjacent OSBs only
    chains = [
        [1, 2, 3, 4, 5],
        [6, 7, 8, 9, 10],
        [15, 14, 13, 12, 11],
        [20, 19, 18, 17, 16],
    ]
    for chain in chains:
        pts = []
        for oid in chain:
            if oid not in sw:
                continue
            set_net(sw[oid][0], "2", board, "GND")
            p = first_pad(sw[oid][0], "2")
            if p:
                pts.append(pad_xy(p))
        for (x1, y1), (x2, y2) in zip(pts, pts[1:]):
            direct(board, x1, y1, x2, y2, gnd, 0.4)

    # Bottom chain to J_GND
    if 13 in sw:
        p = first_pad(sw[13][0], "2")
        jp = first_pad(jgnd, "1")
        if p and jp:
            direct(board, *pad_xy(p), *pad_xy(jp), gnd, 0.4)

    lines += [
        "| GND | **J_GND** | MCU GND |",
        "",
        "## How to use (no KiCad)",
        "",
        "1. Order the board from the gerber zip (see ORDER-THIS.md).",
        "2. Solder the 6×6 switches and pin headers (or order SMT+THT assembly).",
        "3. Plug Dupont wires: each `J#` → one MCU GPIO, `J_GND` → ground.",
        "4. Firmware: pin low = button pressed (internal pull-up on MCU).",
        "",
    ]
    path.parent.mkdir(parents=True, exist_ok=True)
    (path.parent / "cmfd-board-a-pinmap.md").write_text("\n".join(lines))
    pcbnew.SaveBoard(str(path), board)
    print(f"Wrote {path}")
    return path


def build_board_b(path: Path):
    W, H = 120.0, 90.0
    board = new_board()
    ensure_net(board, "GND")
    edge_rect(board, 0, 0, W, H)
    silk(board, 3, H - 3, "CMFD BOARD-B  CARRIER", 1.1)
    silk(board, 3, H - 6, "SoM / USB-C / ports  |  wire bezel pins here", 0.7)

    for i, (x, y) in enumerate([(8, 8), (W - 8, 8), (W - 8, H - 8), (8, H - 8)], 1):
        place(board, "MountingHole", "MountingHole_3.2mm_M3", f"H{i}", "M3", x, y)

    # 24-pin reception strip for bezel wires (1x20 + 1x4 rockers + space)
    place(board, "Connector_PinHeader_2.54mm", "PinHeader_1x20_P2.54mm_Vertical", "J_BEZEL", "OSB1-20", 55, 18)
    place(board, "Connector_PinHeader_2.54mm", "PinHeader_1x04_P2.54mm_Vertical", "J_RK", "rockers", 100, 18)
    place(board, "Connector_PinHeader_2.54mm", "PinHeader_1x02_P2.54mm_Vertical", "J_GND", "GND", 110, 18)

    place(board, "Connector_PinHeader_2.54mm", "PinHeader_2x10_P2.54mm_Vertical", "J10", "SoM_A", 30, 45)
    place(board, "Connector_PinHeader_2.54mm", "PinHeader_2x10_P2.54mm_Vertical", "J11", "SoM_B", 30, 62)
    place(
        board,
        "Connector_USB",
        "USB_C_Receptacle_GCT_USB4105-xx-A_16P_TopMnt_Horizontal",
        "J3",
        "USB-C",
        75,
        50,
    )
    place(board, "Connector_RJ", "RJ45_Amphenol_RJHSE5380", "J5", "Eth", 95, 70)
    place(board, "Package_TO_SOT_SMD", "SOT-23-5", "U11", "3V3", 70, 72)
    for i in range(4):
        place(board, "Capacitor_SMD", "C_0603_1608Metric", f"C{i+1}", "100nF", 55 + i * 4, 80)

    path.parent.mkdir(parents=True, exist_ok=True)
    pcbnew.SaveBoard(str(path), board)
    print(f"Wrote {path}")
    return path


def main():
    print(f"pcbnew {getattr(pcbnew, 'Version', lambda: '?')()}")
    print(f"footprints: {FP_ROOT}")
    build_board_a(ROOT / "elec" / "bezel-mcu" / "cmfd-board-a.kicad_pcb")
    build_board_b(ROOT / "elec" / "carrier-som" / "cmfd-board-b.kicad_pcb")


if __name__ == "__main__":
    main()

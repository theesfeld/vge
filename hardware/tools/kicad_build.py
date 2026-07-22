#!/usr/bin/env python3
"""
Build CMFD Board A / Board B as native KiCad 10 .kicad_pcb files (headless pcbnew).

Uses stock KiCad footprint libraries from the Nix kicad-footprints package.
Run via:  bash hardware/tools/kicad_export.sh
"""
from __future__ import annotations

import os
import sys
from pathlib import Path

# --- discover pcbnew + footprint libs ---------------------------------------

def _find_fp_root() -> Path:
    env = os.environ.get("KICAD_FOOTPRINT_DIR")
    if env and Path(env).is_dir():
        return Path(env)
    store = Path("/nix/store")
    matches = sorted(store.glob("*-kicad-footprints-*/share/kicad/footprints"))
    if not matches:
        raise SystemExit("No kicad footprints under /nix/store — use nix-shell -p kicad")
    return matches[-1]


def _ensure_pcbnew():
    if "pcbnew" in sys.modules:
        return
    # Prefer nix kicad-base site-packages
    candidates = sorted(Path("/nix/store").glob("*-kicad-base-*/lib/python*/site-packages"))
    for c in candidates:
        if (c / "pcbnew.py").exists() or (c / "_pcbnew.so").exists():
            sys.path.insert(0, str(c))
            break
    import pcbnew  # noqa: F401


_ensure_pcbnew()
import pcbnew  # type: ignore  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
FP_ROOT = _find_fp_root()


def mm(v: float) -> int:
    return pcbnew.FromMM(v)


def vec(x: float, y: float) -> "pcbnew.VECTOR2I":
    return pcbnew.VECTOR2I(mm(x), mm(y))


def load_fp(lib_pretty: str, name: str) -> "pcbnew.FOOTPRINT":
    lib_path = str(FP_ROOT / f"{lib_pretty}.pretty")
    fp = pcbnew.FootprintLoad(lib_path, name)
    if fp is None:
        raise RuntimeError(f"Footprint not found: {lib_pretty}:{name} in {lib_path}")
    return fp


def place(
    board: "pcbnew.BOARD",
    lib: str,
    name: str,
    ref: str,
    value: str,
    x: float,
    y: float,
    rot_deg: float = 0.0,
) -> "pcbnew.FOOTPRINT":
    fp = load_fp(lib, name)
    fp.SetReference(ref)
    fp.SetValue(value)
    fp.SetPosition(vec(x, y))
    if rot_deg:
        # KiCad 10: tenths of a degree in EDA_ANGLE
        fp.SetOrientation(pcbnew.EDA_ANGLE(rot_deg, pcbnew.DEGREES_T))
    board.Add(fp)
    return fp


def add_edge_rect(board: "pcbnew.BOARD", x0: float, y0: float, w: float, h: float, width: float = 0.1):
    pts = [
        (x0, y0),
        (x0 + w, y0),
        (x0 + w, y0 + h),
        (x0, y0 + h),
        (x0, y0),
    ]
    for (x1, y1), (x2, y2) in zip(pts, pts[1:]):
        s = pcbnew.PCB_SHAPE(board)
        s.SetShape(pcbnew.SHAPE_T_SEGMENT)
        s.SetLayer(pcbnew.Edge_Cuts)
        s.SetStart(vec(x1, y1))
        s.SetEnd(vec(x2, y2))
        s.SetWidth(mm(width))
        board.Add(s)


def add_silk_text(board: "pcbnew.BOARD", x: float, y: float, text: str, size: float = 1.0):
    t = pcbnew.PCB_TEXT(board)
    t.SetText(text)
    t.SetPosition(vec(x, y))
    t.SetLayer(pcbnew.F_SilkS)
    t.SetTextSize(pcbnew.VECTOR2I(mm(size), mm(size)))
    t.SetTextThickness(mm(size * 0.15))
    board.Add(t)


def add_zone(
    board: "pcbnew.BOARD",
    netname: str,
    layer,
    outline_mm: list[tuple[float, float]],
    clearance: float = 0.2,
    min_width: float = 0.25,
):
    """Add a copper zone. outline_mm is closed polygon in mm board coords."""
    netcode = board.GetNetcodeFromNetname(netname)
    if netcode < 0:
        # create net
        ni = board.FindNet(netname)
        if ni is None:
            new_net = pcbnew.NETINFO_ITEM(board, netname)
            board.Add(new_net)
            netcode = new_net.GetNetCode()
        else:
            netcode = ni.GetNetCode()

    zone = pcbnew.ZONE(board)
    zone.SetNetCode(netcode)
    zone.SetLayer(layer)
    zone.SetLocalClearance(mm(clearance))
    zone.SetMinThickness(mm(min_width))
    zone.SetPadConnection(pcbnew.ZONE_CONNECTION_THERMAL)
    zone.SetThermalReliefGap(mm(0.3))
    zone.SetThermalReliefSpokeWidth(mm(0.3))
    zone.SetIsFilled(False)

    chain = zone.Outline().NewOutline()
    for i, (x, y) in enumerate(outline_mm):
        if i == 0:
            zone.Outline().SetOutlinePoint(0, 0, vec(x, y))  # may not exist
    # Prefer Append
    zone.Outline().RemoveAllContours()
    zone.Outline().NewOutline()
    for x, y in outline_mm:
        zone.Outline().Append(mm(x), mm(y))

    board.Add(zone)
    return zone


def ensure_net(board: "pcbnew.BOARD", name: str) -> int:
    ni = board.FindNet(name)
    if ni is None:
        ni = pcbnew.NETINFO_ITEM(board, name)
        board.Add(ni)
    return ni.GetNetCode()


def assign_pad_net(fp: "pcbnew.FOOTPRINT", pad_name: str, board: "pcbnew.BOARD", netname: str):
    netcode = ensure_net(board, netname)
    for pad in fp.Pads():
        if pad.GetNumber() == pad_name or pad.GetName() == pad_name:
            pad.SetNetCode(netcode)
            return True
    return False


def assign_all_pads_net(fp: "pcbnew.FOOTPRINT", board: "pcbnew.BOARD", netname: str):
    netcode = ensure_net(board, netname)
    for pad in fp.Pads():
        pad.SetNetCode(netcode)


def connect_pads_track(
    board: "pcbnew.BOARD",
    fp_a: "pcbnew.FOOTPRINT",
    pad_a: str,
    fp_b: "pcbnew.FOOTPRINT",
    pad_b: str,
    netname: str,
    width: float = 0.3,
    layer=None,
):
    if layer is None:
        layer = pcbnew.F_Cu
    netcode = ensure_net(board, netname)
    pa = pb = None
    for pad in fp_a.Pads():
        if pad.GetNumber() == pad_a:
            pa = pad
            pad.SetNetCode(netcode)
    for pad in fp_b.Pads():
        if pad.GetNumber() == pad_b:
            pb = pad
            pad.SetNetCode(netcode)
    if pa is None or pb is None:
        return False
    track = pcbnew.PCB_TRACK(board)
    track.SetStart(pa.GetPosition())
    track.SetEnd(pb.GetPosition())
    track.SetWidth(mm(width))
    track.SetLayer(layer)
    track.SetNetCode(netcode)
    board.Add(track)
    return True


def new_board(title: str) -> "pcbnew.BOARD":
    board = pcbnew.BOARD()
    ds = board.GetDesignSettings()
    ds.SetBoardThickness(mm(1.6))
    # Prototype 2-layer rules aligned with common JLCPCB minimums
    ds.m_TrackMinWidth = mm(0.15)
    ds.m_ViasMinSize = mm(0.45)
    ds.m_ViasMinDrill = mm(0.3)
    ds.m_CopperEdgeClearance = mm(0.25)
    ds.m_HoleClearance = mm(0.15)
    ds.m_HoleToHoleMin = mm(0.25)
    ds.m_SilkClearance = mm(0.1)
    return board


def refill_zones(board: "pcbnew.BOARD"):
    """ZONE_FILLER segfaults under pcbnew SWIG on this host — refill via kicad-cli instead."""
    print("  (zones unfilled here; kicad-cli --refill-zones will fill)")


# --- geometry (matches prior design) ----------------------------------------

def osb_centers(W: float = 140.0, H: float = 140.0, inset: float = 13.5, span: float = 72.0):
    """Return list of (osb_id, x, y) for OSB 1..20.

    Frame width is (140-105)/2 = 17.5 mm. Keep OSB ≥3 mm from outer edge and cutout.
    """
    step = span / 4
    c = W / 2
    out = []
    # top 1-5 L→R
    top_y = H - inset
    for i in range(5):
        out.append((1 + i, c - span / 2 + i * step, top_y))
    # right 6-10 T→B
    right_x = W - inset
    for i in range(5):
        out.append((6 + i, right_x, H / 2 + span / 2 - i * step))
    # bottom physical L→R = 15,14,13,12,11
    bot_y = inset
    for i in range(5):
        out.append((15 - i, c - span / 2 + i * step, bot_y))
    # left T→B = 20,19,18,17,16
    left_x = inset
    for i in range(5):
        out.append((20 - i, left_x, H / 2 + span / 2 - i * step))
    return out


# --- Board A ----------------------------------------------------------------

def build_board_a(path: Path):
    W, H = 140.0, 140.0
    cut = 105.0
    cut_x = (W - cut) / 2
    cut_y = (H - cut) / 2

    board = new_board("CMFD Board A Bezel MCU")
    ensure_net(board, "GND")
    ensure_net(board, "3V3")

    add_edge_rect(board, 0, 0, W, H)
    add_edge_rect(board, cut_x, cut_y, cut, cut)  # display aperture

    add_silk_text(board, 4, H - 3, "CMFD BOARD-A BEZEL MCU  #137", 1.2)
    add_silk_text(board, 4, H - 6, "KiCad headless · STM32G431 · OSB 1-20", 0.9)

    # Mounting holes (clear of OSB courtyards)
    for i, (x, y) in enumerate([(7, 7), (W - 7, 7), (W - 7, H - 7), (7, H - 7)], start=1):
        place(board, "MountingHole", "MountingHole_3.2mm_M3_Pad", f"H{i}", "M3", x, y)

    # MCU + support on bottom frame strip (between bottom OSB and cutout)
    # cut_y ≈ 17.5; bottom OSB at y=12 — place MCU on left of bottom frame mid-band
    u1 = place(
        board,
        "Package_QFP",
        "LQFP-48_7x7mm_P0.5mm",
        "U1",
        "STM32G431CBU6",
        45.0,
        30.0,
    )

    # Decoupling near MCU (spaced for courtyard)
    for i, dx in enumerate([-8, -4, 0, 4, 8]):
        place(board, "Capacitor_SMD", "C_0603_1608Metric", f"C{i+1}", "100nF", 45.0 + dx, 38.0)

    # LDO
    place(board, "Package_TO_SOT_SMD", "SOT-23-5", "U4", "AMS1117-3.3", 62.0, 30.0)

    # OSB switches — H4.3mm 6x6 class, no rotation (courtyards axis-aligned)
    sw_fps = {}
    for oid, x, y in osb_centers(W, H):
        fp = place(
            board,
            "Button_Switch_THT",
            "SW_PUSH_6mm_H4.3mm",
            f"S{oid}",
            f"OSB{oid}",
            x,
            y,
            0.0,
        )
        sw_fps[oid] = fp
        # OSB id is the footprint Value (OSB1..); avoid extra silk near edges

    # Rockers: single tactile per corner (UP/DN via dual-momentary part later)
    # Place in frame corners, clear of OSB and M3
    rockers = [
        ("GAIN", 22.0, H - 22.0),
        ("SYM", W - 22.0, H - 22.0),
        ("BRT", 22.0, 22.0),
        ("CON", W - 22.0, 22.0),
    ]
    for i, (name, x, y) in enumerate(rockers, start=1):
        place(board, "Button_Switch_THT", "SW_PUSH_6mm_H4.3mm", f"SW{i}", name, x, y)
        add_silk_text(board, x - 3.5, y + 7.0, name, 0.75)

    # B2B + SWD on bottom frame (right of MCU cluster)
    place(
        board,
        "Connector_PinHeader_2.54mm",
        "PinHeader_2x10_P2.54mm_Vertical",
        "J1",
        "B2B",
        95.0,
        30.0,
    )
    place(
        board,
        "Connector_PinHeader_2.54mm",
        "PinHeader_1x04_P2.54mm_Vertical",
        "J2",
        "SWD",
        78.0,
        30.0,
    )

    # No copper zones in v1 headless build: zone-to-cutout edge clearance
    # is unreliable under SWIG fill; add GND pour in GUI after fit-check.
    # Switch nets labeled for future routing.
    for oid, fp in sw_fps.items():
        pads = sorted(fp.Pads(), key=lambda p: str(p.GetNumber()))
        if len(pads) >= 2:
            pads[0].SetNetCode(ensure_net(board, f"OSB{oid}"))
            pads[1].SetNetCode(ensure_net(board, "GND"))
            if len(pads) >= 4:
                pads[2].SetNetCode(ensure_net(board, f"OSB{oid}"))
                pads[3].SetNetCode(ensure_net(board, "GND"))

    path.parent.mkdir(parents=True, exist_ok=True)
    pcbnew.SaveBoard(str(path), board)
    print(f"Wrote {path}")
    return path


# --- Board B ----------------------------------------------------------------

def build_board_b(path: Path):
    W, H = 120.0, 90.0
    board = new_board("CMFD Board B SoM Carrier")
    ensure_net(board, "GND")
    ensure_net(board, "3V3")
    ensure_net(board, "5V")
    ensure_net(board, "VBUS")

    add_edge_rect(board, 0, 0, W, H)
    add_silk_text(board, 3, H - 3, "CMFD BOARD-B CARRIER + SoM  #137", 1.2)
    add_silk_text(board, 3, H - 6, "USB-C · Eth · CAN · UART · 18650 · KiCad headless", 0.85)

    for i, (x, y) in enumerate([(6, 6), (W - 6, 6), (W - 6, H - 6), (6, H - 6)], start=1):
        place(board, "MountingHole", "MountingHole_3.2mm_M3_Pad", f"H{i}", "M3", x, y)

    # SoM mezzanine as two 2x10 headers with courtyard gap
    place(
        board,
        "Connector_PinHeader_2.54mm",
        "PinHeader_2x10_P2.54mm_Vertical",
        "J10",
        "SoM_A",
        28,
        40,
    )
    place(
        board,
        "Connector_PinHeader_2.54mm",
        "PinHeader_2x10_P2.54mm_Vertical",
        "J11",
        "SoM_B",
        28,
        58,
    )
    add_silk_text(board, 10, 68, "MOD1 SoM mezzanine (RK3566 class)", 0.8)

    # USB-C along bottom edge, spaced for courtyards
    place(
        board,
        "Connector_USB",
        "USB_C_Receptacle_GCT_USB4105-xx-A_16P_TopMnt_Horizontal",
        "J3",
        "USB-C_PD",
        22,
        12,
    )
    place(
        board,
        "Connector_USB",
        "USB_C_Receptacle_GCT_USB4105-xx-A_16P_TopMnt_Horizontal",
        "J4",
        "USB-C_AUX",
        58,
        12,
    )

    # RJ45
    place(
        board,
        "Connector_RJ",
        "RJ45_Amphenol_RJHSE5380",
        "J5",
        "Ethernet",
        22,
        78,
    )

    # Multi-IO / B2B / audio / bat — spaced grid
    place(
        board,
        "Connector_PinHeader_2.54mm",
        "PinHeader_2x10_P2.54mm_Vertical",
        "J2",
        "B2B_A",
        75,
        48,
    )
    place(
        board,
        "Connector_PinHeader_2.54mm",
        "PinHeader_2x05_P2.54mm_Vertical",
        "J6",
        "CAN_UART",
        95,
        18,
    )
    place(
        board,
        "Connector_PinHeader_2.54mm",
        "PinHeader_1x04_P2.54mm_Vertical",
        "J7",
        "AUDIO",
        110,
        18,
    )
    place(
        board,
        "Connector_PinHeader_2.54mm",
        "PinHeader_1x04_P2.54mm_Vertical",
        "J8",
        "BAT",
        110,
        55,
    )

    # Power / interface ICs
    place(board, "Package_TO_SOT_SMD", "SOT-23-5", "U11", "REG_3V3", 95, 72)
    place(board, "Package_TO_SOT_SMD", "SOT-23-5", "U12", "REG_5V", 108, 72)
    place(board, "Package_SO", "SOIC-8_3.9x4.9mm_P1.27mm", "U6", "ISO1042", 95, 40)

    for i in range(6):
        place(
            board,
            "Capacitor_SMD",
            "C_0603_1608Metric",
            f"C{i+1}",
            "100nF",
            70 + i * 4,
            80,
        )

    # No zones in v1 (USB-C / RJ45 edge copper + SWIG zone fill issues).
    # Pour GND in KiCad GUI after mechanical fit-check.

    path.parent.mkdir(parents=True, exist_ok=True)
    pcbnew.SaveBoard(str(path), board)
    print(f"Wrote {path}")
    return path


def main():
    print(f"pcbnew {pcbnew.Version() if hasattr(pcbnew, 'Version') else '?'}")
    print(f"footprints: {FP_ROOT}")
    a = ROOT / "elec" / "bezel-mcu" / "cmfd-board-a.kicad_pcb"
    b = ROOT / "elec" / "carrier-som" / "cmfd-board-b.kicad_pcb"
    build_board_a(a)
    build_board_b(b)
    smoke = ROOT / "elec" / "bezel-mcu" / "_smoke.kicad_pcb"
    if smoke.exists():
        smoke.unlink()
        print(f"Removed {smoke}")


if __name__ == "__main__":
    main()

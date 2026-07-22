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


def to_mm(v) -> float:
    return pcbnew.ToMM(v)


def pad_xy(pad) -> tuple[float, float]:
    p = pad.GetPosition()
    return (to_mm(p.x), to_mm(p.y))


def first_pad(fp: "pcbnew.FOOTPRINT", number: str):
    for pad in fp.Pads():
        if pad.GetNumber() == number:
            return pad
    pads = list(fp.Pads())
    return pads[0] if pads else None


def set_pads_net(fp: "pcbnew.FOOTPRINT", number: str, board: "pcbnew.BOARD", netname: str):
    netcode = ensure_net(board, netname)
    n = 0
    for pad in fp.Pads():
        if pad.GetNumber() == number:
            pad.SetNetCode(netcode)
            n += 1
    return n


def add_track_mm(
    board: "pcbnew.BOARD",
    x1: float,
    y1: float,
    x2: float,
    y2: float,
    netcode: int,
    width: float = 0.25,
    layer=None,
):
    if layer is None:
        layer = pcbnew.F_Cu
    if abs(x1 - x2) < 1e-6 and abs(y1 - y2) < 1e-6:
        return
    t = pcbnew.PCB_TRACK(board)
    t.SetStart(vec(x1, y1))
    t.SetEnd(vec(x2, y2))
    t.SetWidth(mm(width))
    t.SetLayer(layer)
    t.SetNetCode(netcode)
    board.Add(t)


def add_via_mm(board: "pcbnew.BOARD", x: float, y: float, netcode: int, size: float = 0.6, drill: float = 0.3):
    v = pcbnew.PCB_VIA(board)
    v.SetPosition(vec(x, y))
    v.SetViaType(pcbnew.VIATYPE_THROUGH)
    v.SetWidth(mm(size))
    v.SetDrill(mm(drill))
    v.SetNetCode(netcode)
    # top-bottom
    if hasattr(v, "SetLayerPair"):
        v.SetLayerPair(pcbnew.F_Cu, pcbnew.B_Cu)
    board.Add(v)


def route_polyline(
    board: "pcbnew.BOARD",
    pts: list[tuple[float, float]],
    netcode: int,
    width: float = 0.25,
    layer=None,
):
    if layer is None:
        layer = pcbnew.F_Cu
    for (x1, y1), (x2, y2) in zip(pts, pts[1:]):
        add_track_mm(board, x1, y1, x2, y2, netcode, width, layer)


def route_manhattan(
    board: "pcbnew.BOARD",
    x1: float,
    y1: float,
    x2: float,
    y2: float,
    netcode: int,
    width: float = 0.25,
    layer=None,
    via_mid: bool = False,
):
    """L-shaped route; optional via at corner for layer change."""
    if layer is None:
        layer = pcbnew.F_Cu
    # prefer horizontal first then vertical
    if abs(x1 - x2) > 1e-6 and abs(y1 - y2) > 1e-6:
        add_track_mm(board, x1, y1, x2, y1, netcode, width, layer)
        if via_mid:
            add_via_mm(board, x2, y1, netcode)
            other = pcbnew.B_Cu if layer == pcbnew.F_Cu else pcbnew.F_Cu
            add_track_mm(board, x2, y1, x2, y2, netcode, width, other)
        else:
            add_track_mm(board, x2, y1, x2, y2, netcode, width, layer)
    else:
        add_track_mm(board, x1, y1, x2, y2, netcode, width, layer)


def route_chain_pads(
    board: "pcbnew.BOARD",
    pads: list,
    netname: str,
    width: float = 0.25,
    layer=None,
):
    """Assign net and connect pad centers in order (same layer)."""
    if layer is None:
        layer = pcbnew.F_Cu
    netcode = ensure_net(board, netname)
    pts = []
    for pad in pads:
        pad.SetNetCode(netcode)
        pts.append(pad_xy(pad))
    for (x1, y1), (x2, y2) in zip(pts, pts[1:]):
        # L-route if not axis-aligned enough
        if abs(x1 - x2) > 0.05 and abs(y1 - y2) > 0.05:
            route_manhattan(board, x1, y1, x2, y2, netcode, width, layer)
        else:
            add_track_mm(board, x1, y1, x2, y2, netcode, width, layer)
    return netcode


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
    # Keep M3 clear of COL/ROW channel rings (4.5–12 mm from edge)
    for i, (x, y) in enumerate([(15, 15), (W - 15, 15), (W - 15, H - 15), (15, H - 15)], start=1):
        place(board, "MountingHole", "MountingHole_3.2mm_M3", f"H{i}", "M3", x, y)

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
    rockers_spec = [
        ("GAIN", 22.0, H - 22.0),
        ("SYM", W - 22.0, H - 22.0),
        ("BRT", 22.0, 22.0),
        ("CON", W - 22.0, 22.0),
    ]
    rocker_fps = []
    for i, (name, x, y) in enumerate(rockers_spec, start=1):
        rfp = place(board, "Button_Switch_THT", "SW_PUSH_6mm_H4.3mm", f"SW{i}", name, x, y)
        rocker_fps.append((name, rfp))
        add_silk_text(board, x - 3.5, y + 7.0, name, 0.75)

    # B2B + SWD on bottom frame (right of MCU cluster)
    j1 = place(
        board,
        "Connector_PinHeader_2.54mm",
        "PinHeader_2x10_P2.54mm_Vertical",
        "J1",
        "B2B",
        95.0,
        30.0,
    )
    j2 = place(
        board,
        "Connector_PinHeader_2.54mm",
        "PinHeader_1x04_P2.54mm_Vertical",
        "J2",
        "SWD",
        78.0,
        30.0,
    )

    # ------------------------------------------------------------------
    # OSB matrix 4×5 — all matrix copper on B.Cu buses
    #   F.Cu: dual-pad shorts + via at switch only
    #   B.Cu: ROW buses + COL rings (offset channels) + wrap to bottom
    #   F.Cu bottom strip: fan into MCU only (y < 16)
    # ------------------------------------------------------------------
    side_ids = {
        "TOP": [1, 2, 3, 4, 5],
        "RIGHT": [6, 7, 8, 9, 10],
        "BOT": [15, 14, 13, 12, 11],
        "LEFT": [20, 19, 18, 17, 16],
    }
    row_net = {"TOP": "ROW_TOP", "RIGHT": "ROW_RIGHT", "BOT": "ROW_BOT", "LEFT": "ROW_LEFT"}
    mcu_row_pin = {"TOP": "10", "RIGHT": "11", "BOT": "12", "LEFT": "13"}
    mcu_col_pin = ["14", "15", "16", "17", "18"]
    mcu_rk_pin = {"GAIN": "19", "SYM": "20", "BRT": "21", "CON": "22"}
    w_sig = 0.2

    def mcu_pad(pin: str):
        return first_pad(u1, pin)

    def pads_num(fp, number: str):
        return [p for p in fp.Pads() if p.GetNumber() == number]

    def short_and_via(fp, number: str, netname: str, toward_center: bool = True):
        """Short dual pads on F.Cu, via offset in/out to separate ROW vs COL."""
        netcode = ensure_net(board, netname)
        pads = pads_num(fp, number)
        for p in pads:
            p.SetNetCode(netcode)
        if not pads:
            return None
        if len(pads) >= 2:
            x1, y1 = pad_xy(pads[0])
            x2, y2 = pad_xy(pads[1])
            add_track_mm(board, x1, y1, x2, y2, netcode, w_sig, pcbnew.F_Cu)
            cx, cy = (x1 + x2) / 2, (y1 + y2) / 2
        else:
            cx, cy = pad_xy(pads[0])
        # offset via 1.1 mm toward board center (COL) or outer edge (ROW)
        bx, by = W / 2, H / 2
        import math
        dx, dy = bx - cx, by - cy
        n = math.hypot(dx, dy) or 1.0
        ux, uy = dx / n, dy / n
        if toward_center:
            vx, vy = cx + ux * 1.15, cy + uy * 1.15
        else:
            vx, vy = cx - ux * 1.15, cy - uy * 1.15
        vx = min(max(vx, 2.0), W - 2.0)
        vy = min(max(vy, 2.0), H - 2.0)
        add_track_mm(board, cx, cy, vx, vy, netcode, w_sig, pcbnew.F_Cu)
        add_via_mm(board, vx, vy, netcode, size=0.5, drill=0.25)
        return vx, vy, netcode

    # Channel plan from outer edge (mm): COL 4.5+1*i, ROW 10.0+0.9*i, free outer 2–3 for rockers
    def col_ch(col: int):
        d = 4.5 + col * 1.0
        return d, W - d, d, H - d

    def row_ch(side: str):
        # distinct from COL 4.5–8.5
        idx = {"TOP": 0, "RIGHT": 1, "BOT": 2, "LEFT": 3}[side]
        d = 10.0 + idx * 0.9
        return d, W - d, d, H - d

    # --- COL rings on B.Cu ---
    bot_col_via = {}
    for col in range(5):
        net = f"COL{col}"
        ch_l, ch_r, ch_b, ch_t = col_ch(col)
        pts = []
        for oid, side in [
            (side_ids["TOP"][col], "TOP"),
            (side_ids["RIGHT"][col], "RIGHT"),
            (side_ids["BOT"][col], "BOT"),
            (side_ids["LEFT"][col], "LEFT"),
        ]:
            res = short_and_via(sw_fps[oid], "2", net, toward_center=True)
            if not res:
                continue
            cx, cy, netcode = res
            if side == "TOP":
                rx, ry = cx, ch_t
            elif side == "RIGHT":
                rx, ry = ch_r, cy
            elif side == "BOT":
                rx, ry = cx, ch_b
                bot_col_via[col] = (rx, ry, netcode)
            else:
                rx, ry = ch_l, cy
            add_track_mm(board, cx, cy, rx, ry, netcode, w_sig, pcbnew.B_Cu)
            pts.append((rx, ry, netcode))
        # Daisy-chain column vias on B.Cu (TOP→RIGHT→BOT→LEFT) via frame channels
        if len(pts) >= 2:
            netcode = pts[0][2]
            ordered = [(p[0], p[1]) for p in pts]
            # pts order is TOP, RIGHT, BOT, LEFT
            if len(ordered) == 4:
                tpt, rpt, bpt, lpt = ordered
                route_polyline(board, [tpt, (ch_r, ch_t), rpt], netcode, w_sig, pcbnew.B_Cu)
                route_polyline(board, [rpt, (ch_r, ch_b), bpt], netcode, w_sig, pcbnew.B_Cu)
                route_polyline(board, [bpt, (ch_l, ch_b), lpt], netcode, w_sig, pcbnew.B_Cu)
                # no close TOP-LEFT to avoid double path
            else:
                for a, b in zip(ordered, ordered[1:]):
                    route_manhattan(board, a[0], a[1], b[0], b[1], netcode, w_sig, pcbnew.B_Cu)

    # --- ROW buses on B.Cu ---
    row_join = {}  # side -> point on bus for MCU feed
    for side, ids in side_ids.items():
        net = row_net[side]
        ch_l, ch_r, ch_b, ch_t = row_ch(side)
        bus_pts = []
        netcode = ensure_net(board, net)
        for oid in ids:
            res = short_and_via(sw_fps[oid], "1", net, toward_center=False)
            if not res:
                continue
            cx, cy, _ = res
            if side == "TOP":
                bx, by = cx, ch_t
            elif side == "RIGHT":
                bx, by = ch_r, cy
            elif side == "BOT":
                bx, by = cx, ch_b
            else:
                bx, by = ch_l, cy
            add_track_mm(board, cx, cy, bx, by, netcode, w_sig, pcbnew.B_Cu)
            bus_pts.append((bx, by))
        # stitch bus along side
        if side in ("TOP", "BOT"):
            bus_pts.sort(key=lambda p: p[0])
            y = ch_t if side == "TOP" else ch_b
            for (x1, _), (x2, _) in zip(bus_pts, bus_pts[1:]):
                add_track_mm(board, x1, y, x2, y, netcode, w_sig, pcbnew.B_Cu)
            # join corner toward left for feed
            if bus_pts:
                row_join[side] = (bus_pts[0][0], y, netcode)
        else:
            bus_pts.sort(key=lambda p: p[1])
            x = ch_r if side == "RIGHT" else ch_l
            for (_, y1), (_, y2) in zip(bus_pts, bus_pts[1:]):
                add_track_mm(board, x, y1, x, y2, netcode, w_sig, pcbnew.B_Cu)
            if bus_pts:
                row_join[side] = (x, bus_pts[0][1], netcode)

    # Feed ROW buses to bottom-left then MCU (B.Cu then via to F.Cu)
    for i, side in enumerate(["TOP", "LEFT", "BOT", "RIGHT"]):
        if side not in row_join:
            continue
        jx, jy, netcode = row_join[side]
        mp = mcu_pad(mcu_row_pin[side])
        if not mp:
            continue
        mp.SetNetCode(netcode)
        mx, my = pad_xy(mp)
        # gather at bottom-left corridor on B.Cu
        gather_x = 2.5 + i * 0.7
        gather_y = 5.0 + i * 0.7
        route_polyline(
            board,
            [(jx, jy), (gather_x, jy), (gather_x, gather_y)],
            netcode, w_sig, pcbnew.B_Cu,
        )
        add_via_mm(board, gather_x, gather_y, netcode, size=0.55, drill=0.3)
        route_polyline(
            board,
            [(gather_x, gather_y), (mx, gather_y), (mx, my)],
            netcode, w_sig, pcbnew.F_Cu,
        )

    # COL to MCU from bottom channel
    for col in range(5):
        if col not in bot_col_via:
            continue
        bx, by, netcode = bot_col_via[col]
        mp = mcu_pad(mcu_col_pin[col])
        if not mp:
            continue
        mp.SetNetCode(netcode)
        mx, my = pad_xy(mp)
        fan_y = 4.0 + col * 0.55
        # stay on B.Cu to under MCU then via up
        route_polyline(board, [(bx, by), (bx, fan_y), (mx, fan_y)], netcode, w_sig, pcbnew.B_Cu)
        add_via_mm(board, mx, fan_y, netcode, size=0.55, drill=0.3)
        add_track_mm(board, mx, fan_y, mx, my, netcode, w_sig, pcbnew.F_Cu)

    # Rockers: each gets unique B.Cu lane near edge, fan to MCU without sharing vias
    rk_order = ["GAIN", "BRT", "SYM", "CON"]
    for name, rfp in rocker_fps:
        short_and_via(rfp, "2", "GND", toward_center=True)
        res = short_and_via(rfp, "1", f"RK_{name}", toward_center=False)
        if not res:
            continue
        cx, cy, netcode = res
        mp = mcu_pad(mcu_rk_pin[name])
        if not mp:
            continue
        mp.SetNetCode(netcode)
        mx, my = pad_xy(mp)
        idx = rk_order.index(name) if name in rk_order else 0
        # lanes 1.5–2.4 mm from edge
        edge = 1.5 + idx * 0.3
        gy = 2.0 + idx * 0.55
        if name in ("GAIN", "BRT"):
            route_polyline(board, [(cx, cy), (edge, cy), (edge, gy)], netcode, w_sig, pcbnew.B_Cu)
        else:
            route_polyline(
                board,
                [(cx, cy), (W - edge, cy), (W - edge, gy), (edge, gy)],
                netcode, w_sig, pcbnew.B_Cu,
            )
        add_via_mm(board, edge, gy, netcode, size=0.5, drill=0.25)
        # F.Cu fan below MCU courtyard (y small)
        route_polyline(board, [(edge, gy), (mx, gy), (mx, my)], netcode, w_sig, pcbnew.F_Cu)

    # Power nets labeled (pour GND/3V3 in GUI — long stubs short in this frame)
    gnd = ensure_net(board, "GND")
    v3 = ensure_net(board, "3V3")
    for pin in ("3", "4"):
        p = first_pad(j1, pin)
        if p:
            p.SetNetCode(gnd)
    p = first_pad(j2, "3")
    if p:
        p.SetNetCode(gnd)
    for pin in ("1", "2"):
        p = first_pad(j1, pin)
        if p:
            p.SetNetCode(v3)
    p = first_pad(j2, "4")
    if p:
        p.SetNetCode(v3)
    # SWD / UART: net labels; UART short-route on bottom frame
    for mpin, jpin, net in [("34", "1", "SWDIO"), ("37", "2", "SWCLK")]:
        mp, jp = mcu_pad(mpin), first_pad(j2, jpin)
        if mp and jp:
            nc = ensure_net(board, net)
            mp.SetNetCode(nc)
            jp.SetNetCode(nc)
    for mpin, jpin, net in [("29", "5", "UART_TX"), ("30", "6", "UART_RX")]:
        mp, jp = mcu_pad(mpin), first_pad(j1, jpin)
        if mp and jp:
            nc = ensure_net(board, net)
            mp.SetNetCode(nc)
            jp.SetNetCode(nc)
            # short direct if both on bottom frame
            route_manhattan(board, *pad_xy(mp), *pad_xy(jp), nc, w_sig, pcbnew.F_Cu)

    add_silk_text(board, 4, 42, "MATRIX B.Cu bus + via-at-switch", 0.7)

    map_path = path.parent / "cmfd-board-a-pinmap.md"
    map_path.write_text(
        """# Board A pin map (matrix)

| MCU pin | Net | Function |
|--------:|-----|----------|
| 10 | ROW_TOP | OSB 1–5 |
| 11 | ROW_RIGHT | OSB 6–10 |
| 12 | ROW_BOT | OSB 15–11 L→R |
| 13 | ROW_LEFT | OSB 20–16 T→B |
| 14–18 | COL0–4 | column index on each side |
| 19–22 | RK_* | GAIN SYM BRT CON |
| 29–30 | UART_TX/RX | J1.5 / J1.6 |
| 34 / 37 | SWDIO / SWCLK | J2.1 / J2.2 |

Routing: matrix on **B.Cu** buses; vias at each switch; MCU fanout on bottom **F.Cu**.
Firmware: scan COL drive / ROW sense (or reverse). Debounce ≥ 20 ms.
"""
    )

    path.parent.mkdir(parents=True, exist_ok=True)
    pcbnew.SaveBoard(str(path), board)
    print(f"Wrote {path} (matrix routed)")
    print(f"Wrote {map_path}")
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

#!/usr/bin/env python3
"""
CMFD studio renders from real build geometry.

  - Print STLs: rear shell, front bezel, OSB caps, rockers, battery tray
  - Board A / Board B: procedural geometry matching KiCad layout
  - 18650 cells + tray

Requires: nix-shell -p blender --run 'blender -b -P hardware/tools/blender_render_cmfd.py'

Do not show output until images are inspected.
"""
from __future__ import annotations

import math
from pathlib import Path

import bpy
from mathutils import Euler, Vector

ROOT = Path(__file__).resolve().parents[1]
PRINT = ROOT / "mech" / "print"
OUT = ROOT / "studio"
OUT.mkdir(parents=True, exist_ok=True)

# KiCad / OpenSCAD contract (mm)
OUTER = 148.0
OSB_INSET = 14.0
OSB_SPAN = 78.0
CUT = 102.0
BOARD_A_T = 1.6
BOARD_B_W, BOARD_B_H = 120.0, 90.0
ROCKERS = {"GAIN": (26.0, 122.0), "SYM": (122.0, 122.0), "BRT": (26.0, 26.0), "CON": (122.0, 26.0)}


# ---------------------------------------------------------------------------
# Scene helpers
# ---------------------------------------------------------------------------

def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for block in (bpy.data.meshes, bpy.data.cameras, bpy.data.lights):
        for item in list(block):
            if item.users == 0:
                block.remove(item)
    # keep materials (reused by name); wipe orphan meshes only


def make_mat(name: str, rgba, metallic=0.0, roughness=0.5, transmission=0.0):
    """Saturated catalog materials that survive Filmic + studio lights."""
    m = bpy.data.materials.get(name)
    if m is None:
        m = bpy.data.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    bsdf = nt.nodes.new("ShaderNodeBsdfPrincipled")
    bsdf.location = (0, 0)
    out.location = (300, 0)
    nt.links.new(bsdf.outputs["BSDF"], out.inputs["Surface"])
    r, g, b = rgba[:3]
    a = rgba[3] if len(rgba) > 3 else 1.0
    bsdf.inputs["Base Color"].default_value = (r, g, b, 1.0)
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    if "Specular IOR Level" in bsdf.inputs:
        bsdf.inputs["Specular IOR Level"].default_value = 0.35
    if a < 0.99:
        bsdf.inputs["Alpha"].default_value = a
        if "Transmission Weight" in bsdf.inputs:
            bsdf.inputs["Transmission Weight"].default_value = transmission
        m.blend_method = "HASHED"
    return m


def M():
    """Catalog palette — Walkman / MIL industrial. Keep values dark enough for Filmic."""
    return {
        # classic dark FR4, not mint
        "fr4": make_mat("FR4", (0.04, 0.16, 0.07), 0.0, 0.62),
        "pad": make_mat("Pad", (0.65, 0.48, 0.10), 0.9, 0.38),
        "silks": make_mat("Silk", (0.75, 0.75, 0.72), 0.0, 0.65),
        "shell": make_mat("Shell", (0.12, 0.13, 0.15), 0.12, 0.55),  # charcoal
        "bezel": make_mat("Bezel", (0.32, 0.35, 0.38), 0.45, 0.42),  # cool grey metal
        "blue": make_mat("WalkmanBlue", (0.05, 0.18, 0.42), 0.15, 0.45),
        "cap": make_mat("OsbCap", (0.55, 0.57, 0.60), 0.05, 0.48),
        "rocker": make_mat("Rocker", (0.48, 0.50, 0.53), 0.08, 0.45),
        "switch": make_mat("SwitchBody", (0.03, 0.03, 0.03), 0.08, 0.6),
        "stem": make_mat("SwitchStem", (0.08, 0.08, 0.08), 0.05, 0.55),
        "pin": make_mat("HeaderPin", (0.72, 0.60, 0.18), 0.95, 0.32),
        "lcd": make_mat("LcdBody", (0.015, 0.015, 0.018), 0.35, 0.3),
        "glass": make_mat("CoverGlass", (0.08, 0.10, 0.12, 0.55), 0.0, 0.04, transmission=0.75),
        "cell": make_mat("CellBody", (0.02, 0.02, 0.025), 0.7, 0.4),
        "cell_wrap": make_mat("CellWrap", (0.06, 0.06, 0.07), 0.25, 0.5),
        "cell_pos": make_mat("CellPos", (0.70, 0.35, 0.06), 0.9, 0.32),
        "tray": make_mat("Tray", (0.16, 0.17, 0.19), 0.08, 0.55),
        "som": make_mat("SoM", (0.08, 0.08, 0.09), 0.15, 0.5),
        "shield": make_mat("Shield", (0.42, 0.44, 0.42), 0.8, 0.35),
        "usb": make_mat("UsbC", (0.6, 0.6, 0.62), 0.75, 0.3),
        "rj45": make_mat("RJ45", (0.45, 0.35, 0.10), 0.1, 0.55),
        "floor": make_mat("Floor", (0.06, 0.06, 0.07), 0.04, 0.9),
        "hdr_plastic": make_mat("HdrPlastic", (0.04, 0.04, 0.04), 0.04, 0.6),
    }


def blue_face_ring(z_mm, m, outer=124.0, inner=102.0, t=0.7):
    """Walkman blue face plate with glass aperture (not a solid slab over the LCD)."""
    frame = (outer - inner) / 2.0
    half = outer / 2.0
    rails = [
        ("Blue_N", (outer, frame, t), (0, half - frame / 2, z_mm)),
        ("Blue_S", (outer, frame, t), (0, -(half - frame / 2), z_mm)),
        ("Blue_E", (frame, inner, t), (half - frame / 2, 0, z_mm)),
        ("Blue_W", (frame, inner, t), (-(half - frame / 2), 0, z_mm)),
    ]
    for name, size, loc in rails:
        box_mm(name, size, loc, m["blue"])


def assign(obj, mat):
    if obj is None or mat is None:
        return
    me = obj.data
    if hasattr(me, "materials"):
        me.materials.clear()
        me.materials.append(mat)


def import_stl(path: Path, name: str, mat):
    if not path.exists():
        print("MISSING STL", path)
        return None
    before = set(bpy.data.objects.keys())
    bpy.ops.wm.stl_import(filepath=str(path))
    new = list(set(bpy.data.objects.keys()) - before)
    if not new:
        print("STL import empty", path)
        return None
    obj = bpy.data.objects[new[0]]
    obj.name = name
    # OpenSCAD exports mm → meters
    obj.scale = (0.001, 0.001, 0.001)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    assign(obj, mat)
    return obj


def scene_bounds(objects=None):
    objs = objects or [o for o in bpy.data.objects if o.type == "MESH" and o.name != "Floor" and not o.hide_render]
    if not objs:
        return Vector((-0.05, -0.05, 0)), Vector((0.05, 0.05, 0.05))
    bpy.context.view_layer.update()
    mins = Vector((1e9, 1e9, 1e9))
    maxs = Vector((-1e9, -1e9, -1e9))
    for o in objs:
        for c in o.bound_box:
            w = o.matrix_world @ Vector(c)
            mins = Vector(tuple(min(mins[i], w[i]) for i in range(3)))
            maxs = Vector(tuple(max(maxs[i], w[i]) for i in range(3)))
    return mins, maxs


def center_xy_bottom(obj, z_bottom_mm=0.0):
    """Center object in XY; put its bottom at z_bottom_mm."""
    bpy.context.view_layer.update()
    bb = [obj.matrix_world @ Vector(c) for c in obj.bound_box]
    mins = Vector(tuple(min(v[i] for v in bb) for i in range(3)))
    maxs = Vector(tuple(max(v[i] for v in bb) for i in range(3)))
    cx = (mins.x + maxs.x) * 0.5
    cy = (mins.y + maxs.y) * 0.5
    obj.location.x -= cx
    obj.location.y -= cy
    obj.location.z += z_bottom_mm * 0.001 - mins.z
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.transform_apply(location=True, rotation=False, scale=False)


def box_mm(name, size, loc, mat):
    """Axis-aligned box. size/loc in mm. loc = center."""
    bpy.ops.mesh.primitive_cube_add()
    obj = bpy.context.active_object
    obj.name = name
    obj.dimensions = (size[0] * 0.001, size[1] * 0.001, size[2] * 0.001)
    bpy.ops.object.transform_apply(scale=True)
    obj.location = (loc[0] * 0.001, loc[1] * 0.001, loc[2] * 0.001)
    assign(obj, mat)
    return obj


def cyl_mm(name, r_mm, h_mm, loc, mat, rot=(0, 0, 0)):
    bpy.ops.mesh.primitive_cylinder_add(
        radius=r_mm * 0.001,
        depth=h_mm * 0.001,
        location=(loc[0] * 0.001, loc[1] * 0.001, loc[2] * 0.001),
        vertices=32,
    )
    obj = bpy.context.active_object
    obj.name = name
    obj.rotation_euler = Euler(rot, "XYZ")
    bpy.ops.object.transform_apply(rotation=True)
    assign(obj, mat)
    return obj


# ---------------------------------------------------------------------------
# Layout
# ---------------------------------------------------------------------------

def osb_centers():
    """OSB id, x_mm, y_mm in board coordinates (origin bottom-left)."""
    step = OSB_SPAN / 4.0
    c = OUTER / 2.0
    out = []
    for i in range(5):
        out.append((1 + i, c - OSB_SPAN / 2 + i * step, OUTER - OSB_INSET))
    for i in range(5):
        out.append((6 + i, OUTER - OSB_INSET, c + OSB_SPAN / 2 - i * step))
    for i in range(5):
        out.append((15 - i, c - OSB_SPAN / 2 + i * step, OSB_INSET))
    for i in range(5):
        out.append((20 - i, OSB_INSET, c + OSB_SPAN / 2 - i * step))
    return out


def pin_side_offset(oid, x, y):
    if 1 <= oid <= 5:
        return x, y + 5.5
    if 6 <= oid <= 10:
        return x + 5.5, y
    if 11 <= oid <= 15:
        return x, y - 5.5
    return x - 5.5, y


def to_center(x, y):
    return x - OUTER / 2.0, y - OUTER / 2.0


# ---------------------------------------------------------------------------
# Geometry builders
# ---------------------------------------------------------------------------

def build_board_a(z_mm, m):
    """Passive switch frame: 148×148 FR4 with 102 mm cutout — no boolean."""
    t = BOARD_A_T
    frame = (OUTER - CUT) / 2.0  # 23 mm
    half = OUTER / 2.0
    rails = [
        ("BA_N", (OUTER, frame, t), (0, half - frame / 2, z_mm)),
        ("BA_S", (OUTER, frame, t), (0, -(half - frame / 2), z_mm)),
        ("BA_E", (frame, CUT, t), (half - frame / 2, 0, z_mm)),
        ("BA_W", (frame, CUT, t), (-(half - frame / 2), 0, z_mm)),
    ]
    for name, size, loc in rails:
        box_mm(name, size, loc, m["fr4"])

    z_sw = z_mm + t / 2 + 2.15
    z_pin = z_mm + t / 2 + 4.0
    for oid, x, y in osb_centers():
        cx, cy = to_center(x, y)
        box_mm(f"SW{oid}", (6.0, 6.0, 3.5), (cx, cy, z_sw), m["switch"])
        box_mm(f"ST{oid}", (3.2, 3.2, 1.2), (cx, cy, z_sw + 2.0), m["stem"])
        px, py = pin_side_offset(oid, x, y)
        pcx, pcy = to_center(px, py)
        box_mm(f"PN{oid}", (1.6, 1.6, 8.0), (pcx, pcy, z_pin), m["pin"])
        # pad ring under switch
        box_mm(f"PD{oid}", (7.2, 7.2, 0.1), (cx, cy, z_mm + t / 2 + 0.05), m["pad"])

    for name, (x, y) in ROCKERS.items():
        cx, cy = to_center(x, y)
        box_mm(f"RKSW_{name}", (6.0, 6.0, 3.5), (cx, cy, z_sw), m["switch"])
        box_mm(f"RKST_{name}", (3.2, 3.2, 1.2), (cx, cy, z_sw + 2.0), m["stem"])
        hx = x + (-5.5 if x < 74 else 5.5)
        hy = y + (-5.5 if y < 74 else 5.5)
        box_mm(f"RKPN_{name}", (1.6, 1.6, 8.0), (hx - OUTER / 2, hy - OUTER / 2, z_pin), m["pin"])


def build_board_b(z_mm, m):
    box_mm("BoardB", (BOARD_B_W, BOARD_B_H, 1.6), (0, 0, z_mm), m["fr4"])
    # SoM footprint + shield can
    box_mm("SoM", (45, 32, 2.5), (-12, 8, z_mm + 2.0), m["som"])
    box_mm("Shield", (40, 28, 2.0), (-12, 8, z_mm + 4.0), m["shield"])
    # USB-C
    box_mm("USBC", (9.0, 7.5, 3.2), (28, -32, z_mm + 2.2), m["usb"])
    # RJ45
    box_mm("RJ45", (16, 21, 13), (40, 15, z_mm + 7.0), m["rj45"])
    # 2×20 + 1×20 header banks (bezel wires)
    for i in range(20):
        box_mm(f"H1_{i}", (1.0, 1.0, 8.0), (-40 + i * 2.54, -35, z_mm + 4.5), m["pin"])
        box_mm(f"H1b_{i}", (2.0, 2.2, 2.0), (-40 + i * 2.54, -35, z_mm + 1.5), m["hdr_plastic"])
    for i in range(16):
        box_mm(f"H2_{i}", (1.0, 1.0, 8.0), (-30 + i * 2.54, -28, z_mm + 4.5), m["pin"])
        box_mm(f"H2b_{i}", (2.0, 2.2, 2.0), (-30 + i * 2.54, -28, z_mm + 1.5), m["hdr_plastic"])
    # sensor header
    for i in range(6):
        box_mm(f"H3_{i}", (1.0, 1.0, 6.0), (35 + (i % 3) * 2.54, 35 + (i // 3) * 2.54, z_mm + 3.5), m["pin"])


def build_lcd(z_mm, m):
    box_mm("LcdModule", (102, 102, 3.5), (0, 0, z_mm), m["lcd"])
    # active glass with slight blue-black
    box_mm("LcdActive", (96, 96, 0.6), (0, 0, z_mm + 2.0), m["glass"])
    # flex cable stub
    box_mm("LcdFlex", (20, 8, 0.4), (0, -55, z_mm - 0.5), m["pad"])


def build_battery(z_mm, m, use_tray_stl=True):
    tray = None
    if use_tray_stl:
        tray = import_stl(PRINT / "cmfd-battery-tray.stl", "BatteryTray", m["tray"])
        if tray:
            center_xy_bottom(tray, z_mm)
    if tray is None:
        box_mm("BatteryTray", (42, 78, 20), (0, 0, z_mm + 10), m["tray"])

    # two 18650 cells (Ø18.2 × 65 mm) sitting in tray
    for i, x in enumerate((-10.0, 10.0)):
        # body along +Y
        cyl_mm(f"Cell{i}", 9.0, 65.0, (x, 0, z_mm + 12), m["cell"], rot=(math.pi / 2, 0, 0))
        # positive tip (copper) toward +Y
        cyl_mm(f"CellTip{i}", 4.0, 2.0, (x, 33.5, z_mm + 12), m["cell_pos"], rot=(math.pi / 2, 0, 0))
        # negative end cap
        cyl_mm(f"CellNeg{i}", 8.5, 1.5, (x, -33.0, z_mm + 12), m["cell_wrap"], rot=(math.pi / 2, 0, 0))


def instance_caps(m, z_mm):
    path = PRINT / "cmfd-osb-cap.stl"
    base = import_stl(path, "CapProto", m["cap"])
    if not base:
        return
    center_xy_bottom(base, 0)
    bpy.ops.object.transform_apply(location=True)
    for oid, x, y in osb_centers():
        d = base.copy()
        d.data = base.data.copy()
        d.name = f"Cap{oid}"
        bpy.context.collection.objects.link(d)
        d.location = ((x - OUTER / 2) * 0.001, (y - OUTER / 2) * 0.001, z_mm * 0.001)
        assign(d, m["cap"])
    base.hide_render = True
    base.hide_viewport = True


def instance_rockers(m, z_mm):
    path = PRINT / "cmfd-rocker.stl"
    base = import_stl(path, "RockerProto", m["rocker"])
    if not base:
        return
    center_xy_bottom(base, 0)
    bpy.ops.object.transform_apply(location=True)
    for name, (x, y) in ROCKERS.items():
        d = base.copy()
        d.data = base.data.copy()
        d.name = f"Rocker_{name}"
        bpy.context.collection.objects.link(d)
        d.location = ((x - OUTER / 2) * 0.001, (y - OUTER / 2) * 0.001, z_mm * 0.001)
        assign(d, m["rocker"])
    base.hide_render = True
    base.hide_viewport = True


# ---------------------------------------------------------------------------
# Studio + camera + render
# ---------------------------------------------------------------------------

def setup_studio():
    # mid-dark seamless backdrop (not chalk white, not void black)
    world = bpy.data.worlds.get("World") or bpy.data.worlds.new("World")
    bpy.context.scene.world = world
    world.use_nodes = True
    nt = world.node_tree
    nt.nodes.clear()
    bg = nt.nodes.new("ShaderNodeBackground")
    bg.inputs[0].default_value = (0.12, 0.125, 0.14, 1.0)
    bg.inputs[1].default_value = 0.6
    out = nt.nodes.new("ShaderNodeOutputWorld")
    nt.links.new(bg.outputs[0], out.inputs[0])

    bpy.ops.mesh.primitive_plane_add(size=4.0, location=(0, 0, -0.0005))
    floor = bpy.context.active_object
    floor.name = "Floor"
    assign(floor, M()["floor"])

    # soft key / fill / rim — low energy so FR4 green and charcoal stay readable
    def area(loc, energy, size, color, rot=None):
        bpy.ops.object.light_add(type="AREA", location=loc)
        L = bpy.context.active_object
        L.data.energy = energy
        L.data.size = size
        L.data.color = color
        if rot:
            L.rotation_euler = Euler(rot, "XYZ")
        return L

    area((0.40, -0.45, 0.60), 28, 1.1, (1.0, 0.97, 0.93), (math.radians(48), 0, math.radians(28)))
    area((-0.50, 0.20, 0.45), 12, 1.4, (0.55, 0.65, 0.95), (math.radians(58), 0, math.radians(-32)))
    area((0.05, 0.60, 0.35), 14, 0.7, (0.45, 0.55, 0.95), (math.radians(68), 0, 0))


def aim_camera(loc, target, lens=50):
    bpy.ops.object.camera_add(location=loc)
    cam = bpy.context.active_object
    cam.data.lens = lens
    cam.data.clip_start = 0.001
    cam.data.clip_end = 100.0
    direction = Vector(target) - Vector(loc)
    cam.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.camera = cam
    return cam


def auto_frame(azimuth_deg=40, elev_deg=32, pad=2.2, lens=50, target_bias=(0, 0, 0)):
    """Place camera from mesh bounds. pad ~2.0–2.6 for full product; ~1.6 for parts."""
    mins, maxs = scene_bounds()
    center = (mins + maxs) * 0.5
    center = Vector((
        center.x + target_bias[0] * 0.001,
        center.y + target_bias[1] * 0.001,
        center.z + target_bias[2] * 0.001,
    ))
    size = maxs - mins
    # use max axis so thin flat boards still get enough distance
    extent = max(size.x, size.y, size.z, 0.02)
    radius = extent * 0.85 * pad
    az = math.radians(azimuth_deg)
    el = math.radians(elev_deg)
    loc = center + Vector((
        radius * math.cos(el) * math.sin(az),
        -radius * math.cos(el) * math.cos(az),
        radius * math.sin(el),
    ))
    loc.z = max(loc.z, center.z + size.z * 0.2 + 0.02)
    aim_camera(tuple(loc), tuple(center), lens=lens)
    print(f"  frame center={tuple(round(c*1000,1) for c in center)} mm "
          f"size={tuple(round(s*1000,1) for s in size)} mm cam={tuple(round(c*1000,1) for c in loc)} mm pad={pad}")


def render_still(path: Path, res=(2400, 1800), samples=64):
    sc = bpy.context.scene
    sc.render.engine = "CYCLES"
    sc.cycles.samples = samples
    sc.cycles.use_denoising = True
    sc.cycles.device = "CPU"
    # prefer GPU if available
    prefs = bpy.context.preferences.addons.get("cycles")
    if prefs:
        cprefs = prefs.preferences
        try:
            cprefs.compute_device_type = "CUDA"
            for dev in cprefs.devices:
                dev.use = True
            sc.cycles.device = "GPU"
        except Exception:
            sc.cycles.device = "CPU"

    sc.render.resolution_x = res[0]
    sc.render.resolution_y = res[1]
    sc.render.resolution_percentage = 100
    sc.render.filepath = str(path)
    sc.render.image_settings.file_format = "PNG"
    sc.render.image_settings.color_mode = "RGB"
    sc.view_settings.view_transform = "Filmic"
    sc.view_settings.look = "Medium High Contrast"
    sc.view_settings.exposure = -0.35
    sc.view_settings.gamma = 1.0
    sc.render.film_transparent = False

    nmesh = sum(1 for o in bpy.data.objects if o.type == "MESH" and not o.hide_render)
    print(f"RENDER {path.name} meshes={nmesh} samples={samples} device={sc.cycles.device}")
    bpy.ops.render.render(write_still=True)
    print(f"WROTE {path} ({path.stat().st_size if path.exists() else 0} bytes)")


# ---------------------------------------------------------------------------
# Shots
# ---------------------------------------------------------------------------

def shot_exploded(m):
    clear_scene()
    setup_studio()
    m = M()
    rear = import_stl(PRINT / "cmfd-rear-shell.stl", "Rear", m["shell"])
    if rear:
        center_xy_bottom(rear, 0)
    build_battery(8, m)
    build_board_b(38, m)
    build_lcd(58, m)
    build_board_a(68, m)
    front = import_stl(PRINT / "cmfd-front-bezel.stl", "Front", m["bezel"])
    if front:
        center_xy_bottom(front, 92)
    blue_face_ring(105, m, outer=120, inner=102, t=0.8)
    instance_caps(m, 110)
    instance_rockers(m, 112)
    auto_frame(azimuth_deg=42, elev_deg=26, pad=2.6, lens=45)
    render_still(OUT / "render-exploded.png", (2560, 1920), 72)


def shot_closed(m):
    clear_scene()
    setup_studio()
    m = M()
    rear = import_stl(PRINT / "cmfd-rear-shell.stl", "Rear", m["shell"])
    if rear:
        center_xy_bottom(rear, 0)
    build_battery(6, m)
    build_board_b(16, m)
    build_lcd(28, m)
    build_board_a(30, m)
    front = import_stl(PRINT / "cmfd-front-bezel.stl", "Front", m["bezel"])
    if front:
        center_xy_bottom(front, 34)
    blue_face_ring(47.5, m, outer=124, inner=102, t=0.6)
    box_mm("Glass", (100, 100, 0.8), (0, 0, 48.2), m["glass"])
    instance_caps(m, 48.5)
    instance_rockers(m, 49.5)
    auto_frame(azimuth_deg=48, elev_deg=28, pad=2.4, lens=50)
    render_still(OUT / "render-closed.png", (2560, 1920), 72)


def shot_board_a(m):
    clear_scene()
    setup_studio()
    m = M()
    build_lcd(0.0, m)
    build_board_a(3.5, m)
    auto_frame(azimuth_deg=38, elev_deg=42, pad=1.9, lens=48)
    render_still(OUT / "render-board-a-lcd.png", (2400, 1800), 56)


def shot_board_b(m):
    clear_scene()
    setup_studio()
    m = M()
    build_board_b(0.0, m)
    auto_frame(azimuth_deg=45, elev_deg=40, pad=2.0, lens=48)
    render_still(OUT / "render-board-b.png", (2400, 1800), 56)


def shot_battery(m):
    clear_scene()
    setup_studio()
    m = M()
    build_battery(0.0, m, use_tray_stl=True)
    # high elev so tray + both 18650 cells read as a pack, not wall stripes
    auto_frame(azimuth_deg=55, elev_deg=48, pad=2.4, lens=50)
    render_still(OUT / "render-battery.png", (2200, 1650), 48)


def shot_case(m):
    clear_scene()
    setup_studio()
    m = M()
    rear = import_stl(PRINT / "cmfd-rear-shell.stl", "Rear", m["shell"])
    if rear:
        center_xy_bottom(rear, 0)
    front = import_stl(PRINT / "cmfd-front-bezel.stl", "Front", m["bezel"])
    if front:
        center_xy_bottom(front, 50)
    auto_frame(azimuth_deg=50, elev_deg=24, pad=2.5, lens=45)
    render_still(OUT / "render-case.png", (2400, 1800), 56)


def shot_buttons(m):
    clear_scene()
    setup_studio()
    m = M()
    # three OSB caps in a row + one rocker for catalog
    for i, x in enumerate((-22.0, 0.0, 22.0)):
        cap = import_stl(PRINT / "cmfd-osb-cap.stl", f"OsbCap{i}", m["cap"])
        if cap:
            center_xy_bottom(cap, 4)
            cap.location.x = x * 0.001
            cap.location.y = 8 * 0.001
        box_mm(f"SW_u{i}", (6, 6, 3.5), (x, 8, 1.5), m["switch"])
    rock = import_stl(PRINT / "cmfd-rocker.stl", "Rocker", m["rocker"])
    if rock:
        center_xy_bottom(rock, 4)
        rock.location.x = 0
        rock.location.y = -14 * 0.001
    box_mm("SW_rk", (6, 6, 3.5), (0, -14, 1.5), m["switch"])
    auto_frame(azimuth_deg=30, elev_deg=35, pad=2.3, lens=55)
    render_still(OUT / "render-buttons.png", (2200, 1650), 48)


def shot_front_detail(m):
    """Closed unit, 3/4 front — full face with OSB ring + dark glass + rockers."""
    clear_scene()
    setup_studio()
    m = M()
    rear = import_stl(PRINT / "cmfd-rear-shell.stl", "Rear", m["shell"])
    if rear:
        center_xy_bottom(rear, 0)
    build_lcd(28, m)
    build_board_a(30, m)
    front = import_stl(PRINT / "cmfd-front-bezel.stl", "Front", m["bezel"])
    if front:
        center_xy_bottom(front, 34)
    blue_face_ring(47.5, m, outer=124, inner=102, t=0.6)
    box_mm("Glass", (100, 100, 0.8), (0, 0, 48.2), m["glass"])
    instance_caps(m, 48.5)
    instance_rockers(m, 49.5)
    auto_frame(azimuth_deg=28, elev_deg=38, pad=2.0, lens=52)
    render_still(OUT / "render-front-detail.png", (2560, 1920), 72)


def main():
    print("=== CMFD Blender studio ===")
    print("PRINT", PRINT, "exists", PRINT.exists())
    for p in sorted(PRINT.glob("*.stl")):
        print(" ", p.name, p.stat().st_size)
    m = M()  # seed materials
    shot_exploded(m)
    shot_closed(m)
    shot_front_detail(m)
    shot_board_a(m)
    shot_board_b(m)
    shot_battery(m)
    shot_case(m)
    shot_buttons(m)
    print("=== ALL SHOTS DONE ===")
    for p in sorted(OUT.glob("render-*.png")):
        print(f"  {p.name:30s} {p.stat().st_size:10d}")


if __name__ == "__main__":
    main()

import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scratch' / 'solver_site'))
sys.path.insert(0, str(ROOT))

import numpy as np

import garment_solver

MASS_SCALE = 1000.0
GRAVITY = 9.81
TAN_41_5 = math.tan(math.radians(41.5))
WEIGHT_GSM = 150.0
BEND_CM = (1.5, 2.5, 4.0)
SPACINGS = (0.005, 0.0025)
STRIP_WIDTH = 0.025
PLATFORM = 0.03
SETTLE_FRAMES = 150
SUBSTEPS = 4
ITERATIONS = 8
HINGE_SCALE = 3.0


def flexural_rigidity(bend_cm):
    return WEIGHT_GSM / 1000.0 * GRAVITY * (bend_cm / 100.0) ** 3


def build_strip(spacing, overhang):
    length = PLATFORM + overhang
    nx = int(round(length / spacing)) + 1
    ny = int(round(STRIP_WIDTH / spacing)) + 1
    xs = np.linspace(-PLATFORM, overhang, nx)
    ys = np.linspace(0.0, STRIP_WIDTH, ny)
    positions = np.zeros((nx * ny, 3), dtype=np.float32)
    for j, y in enumerate(ys):
        for i, x in enumerate(xs):
            positions[j * nx + i] = (x, y, 0.0)
    triangles = []
    for j in range(ny - 1):
        for i in range(nx - 1):
            a = j * nx + i
            b = a + 1
            c = a + nx
            d = c + 1
            triangles.append((a, b, d))
            triangles.append((a, d, c))
    triangles = np.array(triangles, dtype=np.uint32).ravel()
    return positions, triangles, nx, ny


def build_constraints(positions, triangles):
    edges = set()
    tri = triangles.reshape(-1, 3)
    for a, b, c in tri:
        edges.add((min(a, b), max(a, b)))
        edges.add((min(b, c), max(b, c)))
        edges.add((min(c, a), max(c, a)))
    edge_pairs = np.array(sorted(edges), dtype=np.uint32)
    rest_length = np.linalg.norm(positions[edge_pairs[:, 1]] - positions[edge_pairs[:, 0]], axis=1)
    return edge_pairs, rest_length.astype(np.float32)


def dihedral(positions, a, b, c, d):
    e = positions[b] - positions[a]
    elen = np.linalg.norm(e)
    n1 = np.cross(e, positions[c] - positions[a])
    n2 = np.cross(e, positions[d] - positions[a])
    n1n = n1 / np.linalg.norm(n1)
    n2n = n2 / np.linalg.norm(n2)
    s = np.dot(e, np.cross(n1n, n2n)) / elen
    return float(math.atan2(-s, -float(np.dot(n1n, n2n))))


def hinge_compliance(positions, a, b, c, d, rigidity):
    e = positions[b] - positions[a]
    elen = float(np.linalg.norm(e))
    h1 = float(np.linalg.norm(np.cross(e, positions[c] - positions[a]))) / elen
    h2 = float(np.linalg.norm(np.cross(e, positions[d] - positions[a]))) / elen
    return 1.0 / (HINGE_SCALE * rigidity * elen / (h1 + h2) * MASS_SCALE)


def build_hinges(positions, triangles, rigidity):
    tri = triangles.reshape(-1, 3)
    edge_tris = {}
    for index, (a, b, c) in enumerate(tri):
        for u, v in ((a, b), (b, c), (c, a)):
            key = (min(u, v), max(u, v))
            edge_tris.setdefault(key, []).append(index)
    hinges = []
    rest = []
    for (a, b), tris in edge_tris.items():
        if len(tris) != 2:
            continue
        first = tri[tris[0]]
        second = tri[tris[1]]
        c = next(int(v) for v in first if v != a and v != b)
        d = next(int(v) for v in second if v != a and v != b)
        hinges.append((a, b, c, d))
        rest.append(dihedral(positions, a, b, c, d))
    hinges = np.array(hinges, dtype=np.uint32).ravel()
    compliance = np.array([hinge_compliance(positions, a, b, c, d, rigidity) for a, b, c, d in hinges.reshape(-1, 4)],
                          dtype=np.float32)
    return hinges, np.array(rest, dtype=np.float32), compliance


def clamp_plane_collider(overhang):
    reach = overhang * 2.0
    width = STRIP_WIDTH * 3.0
    drop = reach * TAN_41_5
    positions = np.array([
        (0.0, -width, 0.0),
        (reach, -width, -drop),
        (0.0, width, 0.0),
        (reach, width, -drop),
    ], dtype=np.float32)
    triangles = np.array([0, 1, 2, 1, 3, 2], dtype=np.uint32)
    return positions, triangles


def run_cantilever(spacing, bend_cm, overhang):
    rigidity = flexural_rigidity(bend_cm)
    positions, triangles, nx, ny = build_strip(spacing, overhang)
    edges, rest_length = build_constraints(positions, triangles)
    hinges, hinge_rest, hinge_compliance = build_hinges(positions, triangles, rigidity)

    n = len(positions)
    density = WEIGHT_GSM / 1000.0
    mass = density * spacing * spacing * MASS_SCALE
    inverse_mass = np.full(n, 1.0 / mass, dtype=np.float32)
    compliance = np.full(len(edges), 1e-6, dtype=np.float32)
    partner = np.full(n, -1, dtype=np.int32)
    contact_weight = np.full(n, 1.0, dtype=np.float32)

    params = {
        'frame_time': 1.0 / 30.0,
        'elapsed': 0.0,
        'seam_seconds': 0.8,
        'gravity_x': 0.0,
        'gravity_y': 0.0,
        'gravity_z': -GRAVITY,
        'damping': 1.0,
        'body_margin': 0.001,
        'body_grip': 0.8,
        'body_step': 0.01,
        'body_band': 0.03,
        'self_thickness': 0.0,
        'weld_reach': 0.0,
        'weld_gain': 0.25,
        'max_step': 0.015,
    }
    solver = garment_solver.Solver(
        positions.ravel(),
        inverse_mass,
        edges.ravel(),
        rest_length,
        rest_length.copy(),
        compliance,
        triangles,
        positions.ravel(),
        partner,
        contact_weight,
        params,
        hinges,
        hinge_rest,
        hinge_compliance,
    )
    plane_pos, plane_tri = clamp_plane_collider(overhang)
    solver.set_collider(plane_pos, plane_tri)
    mask = np.zeros(n, dtype=np.uint8)
    mask[np.where(positions[:, 0] <= 0.0)[0]] = 1
    solver.set_pins(mask, positions.ravel())

    for frame in range(SETTLE_FRAMES):
        solver.set_params({'elapsed': frame / 30.0})
        solver.step(SUBSTEPS, ITERATIONS)
    result = np.frombuffer(solver.positions(), dtype=np.float32).reshape(-1, 3)
    return result, nx, ny


def chord_angle(result, nx, ny):
    tip = result[nx - 1::nx]
    tip_x = float(tip[:, 0].mean())
    tip_z = float(tip[:, 2].mean())
    return math.degrees(math.atan2(-tip_z, tip_x))


def find_overhang(spacing, bend_cm):
    target = 2.0 * bend_cm / 100.0
    low = target * 0.5
    high = target * 1.0
    for _ in range(14):
        mid = 0.5 * (low + high)
        result, nx, ny = run_cantilever(spacing, bend_cm, mid)
        angle = chord_angle(result, nx, ny)
        if angle < 41.5:
            low = mid
        else:
            high = mid
    return 0.5 * (low + high)


def main():
    print(f'HINGE_SCALE={HINGE_SCALE} WEIGHT_GSM={WEIGHT_GSM}')
    for spacing in SPACINGS:
        for bend_cm in BEND_CM:
            target = 2.0 * bend_cm / 100.0
            actual = find_overhang(spacing, bend_cm)
            ratio = actual / target
            flag = 'OK' if 0.90 <= ratio <= 1.10 else 'ADJUST'
            print(f'spacing={spacing * 1000:.1f}mm bend={bend_cm}cm target={target * 100:.2f}cm '
                  f'overhang={actual * 100:.2f}cm ratio={ratio:.3f} {flag}')


if __name__ == '__main__':
    main()

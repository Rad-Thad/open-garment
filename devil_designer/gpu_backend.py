from collections import defaultdict

import math

import bpy
import numpy as np

BODY_BAND = .03
WELD_GAIN = .25
SEAM_SECONDS = .8
DAMPING = 1.0
ITERATIONS = 2
LIVE_VERTEX_BUDGET = 600000
INPUT_PREFIX = 'dd_in:'
HINGE_SCALE = 3.3

_sessions = {}
_last_frames = {}


class Slot:
    def __init__(self, obj, name):
        self.obj = obj
        self.name = name

    @property
    def key(self):
        return INPUT_PREFIX + self.name

    @property
    def value(self):
        return self.obj.get(self.key)

    @value.setter
    def value(self, value):
        if isinstance(value, tuple):
            self.obj[self.key] = tuple(float(component) for component in value)
        else:
            self.obj[self.key] = value


def owns(obj):
    return obj.get('dd_solver') == 'GPU'


def prepare(obj):
    try:
        import garment_solver
    except ImportError:
        raise ValueError('GPU solver module is missing from this build.')
    try:
        garment_solver.adapter_name()
    except RuntimeError as error:
        raise ValueError('GPU solver unavailable: ' + str(error) + ' Choose Solver: Nodes in Garment & View.')
    obj['dd_solver'] = 'GPU'


def invalidate(obj):
    _sessions.pop(obj.name, None)


def _vertex_attr(mesh, name, dtype=np.float32, vector=False):
    data = mesh.attributes[name].data
    if vector:
        values = np.empty(len(data) * 3, dtype=dtype)
        data.foreach_get('vector', values)
        return values.reshape(-1, 3)
    values = np.empty(len(data), dtype=dtype)
    data.foreach_get('value', values)
    return values


def _write_positions(obj, positions):
    obj.data.vertices.foreach_set('co', np.ascontiguousarray(positions).ravel())
    obj.data.update()


def _hinge_dihedral(a, b, c, d, rest):
    e = rest[b] - rest[a]
    elen = np.linalg.norm(e)
    n1 = np.cross(e, rest[c] - rest[a])
    n2 = np.cross(e, rest[d] - rest[a])
    n1n = n1 / np.linalg.norm(n1)
    n2n = n2 / np.linalg.norm(n2)
    s = np.dot(e, np.cross(n1n, n2n)) / elen
    return float(math.atan2(-s, -float(np.dot(n1n, n2n))))


def _hinge_compliance(a, b, c, d, rest_position, flex_warp, flex_weft, flex_bias):
    from . import xpbd
    e = rest_position[b] - rest_position[a]
    elen = float(np.linalg.norm(e))
    ex = float(e[0])
    ey = float(e[1])
    denom = ex * ex + ey * ey
    a_ratio = ex * ex / denom
    l_ratio = ey * ey / denom
    bw = float((flex_warp[a] + flex_warp[b] + flex_warp[c] + flex_warp[d]) * 0.25)
    bf = float((flex_weft[a] + flex_weft[b] + flex_weft[c] + flex_weft[d]) * 0.25)
    bb = float((flex_bias[a] + flex_bias[b] + flex_bias[c] + flex_bias[d]) * 0.25)
    b_dir = max(bw * a_ratio * a_ratio + bf * l_ratio * l_ratio + (4 * bb - bw - bf) * a_ratio * l_ratio, 1e-3 * bw)
    h1 = float(np.linalg.norm(np.cross(e, rest_position[c] - rest_position[a]))) / elen
    h2 = float(np.linalg.norm(np.cross(e, rest_position[d] - rest_position[a]))) / elen
    return 1.0 / (HINGE_SCALE * b_dir * elen / (h1 + h2) * xpbd.MASS_SCALE)


def session(scene, obj):
    from . import xpbd
    cached = _sessions.get(obj.name)
    if cached is not None and cached['count'] == len(obj.data.vertices) and cached['signature'] == obj.get('dd_signature'):
        return cached
    mesh = obj.data
    count = len(mesh.vertices)
    positions = np.empty(count * 3, dtype=np.float32)
    mesh.vertices.foreach_get('co', positions)
    positions = positions.reshape(-1, 3)
    edges = np.empty(len(mesh.edges) * 2, dtype=np.int32)
    mesh.edges.foreach_get('vertices', edges)
    edges = edges.reshape(-1, 2)
    mesh.calc_loop_triangles()
    triangles = np.empty(len(mesh.loop_triangles) * 3, dtype=np.int32)
    mesh.loop_triangles.foreach_get('vertices', triangles)
    triangles = triangles.reshape(-1, 3)

    rest_length = _vertex_attr(mesh, 'rest_length')
    seam_start = _vertex_attr(mesh, 'dd_seam_start')
    seam = _vertex_attr(mesh, 'dd_seam', bool)
    edge_compliance = _vertex_attr(mesh, 'dd_edge_compliance')
    mass = _vertex_attr(mesh, 'mass')
    contact_weight = _vertex_attr(mesh, 'dd_contact_weight')
    partner = _vertex_attr(mesh, 'dd_partner', np.int32)
    pinned = _vertex_attr(mesh, 'dd_pinned', bool)
    tacked = _vertex_attr(mesh, 'dd_tacked', bool)
    pin_target = _vertex_attr(mesh, 'dd_pin_target', vector=True)
    tack_vertices = _vertex_attr(mesh, 'dd_tack_vertices', vector=True)
    tack_weights = _vertex_attr(mesh, 'dd_tack_weights', vector=True)
    rest_position = _vertex_attr(mesh, 'dd_rest_position', vector=True)
    bend_rest = _vertex_attr(mesh, 'dd_bend_rest', vector=True)
    bend_compliance = _vertex_attr(mesh, 'dd_bend_compliance')
    flex_warp = _vertex_attr(mesh, 'dd_flex_warp')
    flex_weft = _vertex_attr(mesh, 'dd_flex_weft')
    flex_bias = _vertex_attr(mesh, 'dd_flex_bias')

    edge_tris = defaultdict(list)
    for tri_index, tri in enumerate(triangles):
        for k in range(3):
            a = int(tri[k])
            b = int(tri[(k + 1) % 3])
            edge_tris[(min(a, b), max(a, b))].append(tri_index)
    bend_pairs = []
    bend_rests = []
    bend_comps = []
    hinges = []
    hinge_rests = []
    hinge_comps = []
    for (a, b), tris in edge_tris.items():
        if len(tris) != 2:
            continue
        first = triangles[tris[0]]
        second = triangles[tris[1]]
        c = next(int(v) for v in first if v != a and v != b)
        d = next(int(v) for v in second if v != a and v != b)
        if flex_warp[a] > 0 and flex_warp[b] > 0 and flex_warp[c] > 0 and flex_warp[d] > 0:
            hinges.append((a, b, c, d))
            hinge_rests.append(_hinge_dihedral(a, b, c, d, bend_rest))
            hinge_comps.append(_hinge_compliance(a, b, c, d, rest_position, flex_warp, flex_weft, flex_bias))
        else:
            bend_pairs.append((c, d))
            bend_rests.append(float(np.linalg.norm(bend_rest[c] - bend_rest[d])))
            bend_comps.append(float((bend_compliance[c] + bend_compliance[d]) * 0.5))

    pairs = np.concatenate((edges, np.array(bend_pairs, dtype=np.int32))) if bend_pairs else edges
    rest = np.concatenate((rest_length, np.array(bend_rests, dtype=np.float32)))
    rest_start = np.concatenate((np.where(seam, seam_start, rest_length).astype(np.float32), np.array(bend_rests, dtype=np.float32)))
    compliance = np.concatenate((edge_compliance, np.array(bend_comps, dtype=np.float32)))
    inverse_mass = (1.0 / np.maximum(mass, 1e-9)).astype(np.float32)

    body = Slot(obj, 'Body').value
    tack_target = np.zeros((count, 3), dtype=np.float32)
    collider_positions = np.empty(0, dtype=np.float32)
    collider_triangles = np.empty(0, dtype=np.uint32)
    if body is not None and body.type == 'MESH':
        body_mesh = body.data
        body_mesh.calc_loop_triangles()
        collider_positions = np.empty(len(body_mesh.vertices) * 3, dtype=np.float32)
        body_mesh.vertices.foreach_get('co', collider_positions)
        body_co = collider_positions.reshape(-1, 3)
        collider_triangles = np.empty(len(body_mesh.loop_triangles) * 3, dtype=np.int32)
        body_mesh.loop_triangles.foreach_get('vertices', collider_triangles)
        for k in range(3):
            indices = tack_vertices[:, k].astype(np.int32)
            tack_target += body_co[indices] * tack_weights[:, k:k + 1]

    unit = max(1e-6, scene.unit_settings.scale_length)
    gravity = tuple(float(component) for component in Slot(obj, 'Gravity').value)
    body_band = BODY_BAND / unit
    params = {
        'frame_time': 1.0 / xpbd.SIM_FPS,
        'elapsed': 0.0,
        'seam_seconds': float(Slot(obj, 'Seam Seconds').value),
        'gravity_x': gravity[0],
        'gravity_y': gravity[1],
        'gravity_z': gravity[2],
        'damping': float(Slot(obj, 'Damping').value),
        'body_margin': scene.dd_skin_offset / unit,
        'body_grip': scene.dd_friction,
        'body_step': xpbd.BODY_STEP / unit,
        'body_band': body_band,
        'self_thickness': xpbd.CONTACT_RADIUS / unit if scene.dd_self_collision else 0.0,
        'weld_reach': xpbd.WELD_REACH / unit,
        'weld_gain': WELD_GAIN,
        'max_step': body_band / 2.0,
    }
    import garment_solver
    solver = garment_solver.Solver(
        np.ascontiguousarray(positions.ravel()),
        inverse_mass,
        pairs.astype(np.uint32).ravel(),
        rest,
        rest_start,
        compliance,
        triangles.astype(np.uint32).ravel(),
        np.ascontiguousarray(rest_position.ravel()),
        partner,
        contact_weight,
        params,
        np.array(hinges, dtype=np.uint32).ravel() if hinges else None,
        np.array(hinge_rests, dtype=np.float32) if hinges else None,
        np.array(hinge_comps, dtype=np.float32) if hinges else None,
    )
    if collider_triangles.size:
        solver.set_collider(collider_positions, collider_triangles.astype(np.uint32).ravel())

    cached = {
        'count': count,
        'signature': obj.get('dd_signature'),
        'solver': solver,
        'initial': positions.copy(),
        'current': positions.copy(),
        'rest': rest_position,
        'pinned': pinned,
        'tacked': tacked,
        'pin_target': pin_target,
        'tack_target': tack_target,
        'stepped': False,
    }
    _sessions[obj.name] = cached
    return cached


@bpy.app.handlers.persistent
def advance(scene):
    from . import garment, xpbd
    for obj in garment.generated(scene, 'simulation'):
        if not owns(obj) or not obj.visible_get():
            continue
        if scene.frame_current <= scene.frame_start:
            cached = _sessions.get(obj.name)
            if cached is not None and cached['stepped']:
                _write_positions(obj, cached['initial'])
                _sessions.pop(obj.name, None)
            continue
        if _last_frames.get(scene.name) == scene.frame_current:
            continue
        _last_frames[scene.name] = scene.frame_current
        cached = session(scene, obj)
        solver = cached['solver']
        elapsed = (scene.frame_current - 1 - scene.frame_start) / xpbd.SIM_FPS
        ramp = min(max(elapsed / SEAM_SECONDS, 0.0), 1.0)
        targets = cached['pin_target'].copy()
        tacked = cached['tacked']
        targets[tacked] = cached['pin_target'][tacked] * (1.0 - ramp) + cached['tack_target'][tacked] * ramp
        mask = cached['pinned'].copy()
        grab_index = int(Slot(obj, 'Grab Index').value)
        if grab_index >= 0:
            handle = Slot(obj, 'Grab Handle').value
            radius = max(float(Slot(obj, 'Grab Radius').value), 0.001)
            stride = float(Slot(obj, 'Grab Stride').value)
            reach = max(float(Slot(obj, 'Grab Reach').value), 0.001)
            current = cached['current']
            rest = cached['rest']
            handle_position = np.array(handle.location, dtype=np.float32)
            grabbed = current[grab_index]
            distance = np.linalg.norm(rest - rest[grab_index], axis=1)
            falloff = 1.0 - distance / radius
            smooth = np.maximum(falloff, 0.0) ** 2
            to_hand = handle_position - grabbed
            hand_distance = float(np.linalg.norm(to_hand))
            grip = min(max(1.0 - hand_distance / reach, 0.0), 1.0)
            pull = to_hand / max(hand_distance, 1e-12) * (min(hand_distance, stride) * grip)
            held = smooth > 0.001
            targets[held] = current[held] + pull * smooth[held, None]
            mask[held] = True
        solver.set_pins(mask.astype(np.uint8), np.ascontiguousarray(targets.ravel()))
        solver.set_params({'elapsed': elapsed})
        substeps = int(Slot(obj, 'Inner Substeps').value)
        iterations = int(Slot(obj, 'Iterations').value)
        solver.step(substeps, iterations)
        positions = np.frombuffer(solver.positions(), dtype=np.float32).reshape(-1, 3)
        if not np.isfinite(positions).all():
            _sessions.pop(obj.name, None)
            scene['dd_last_error'] = 'GPU solver diverged; simulation stopped.'
            continue
        _write_positions(obj, positions)
        cached['current'] = positions
        cached['stepped'] = True


def register():
    bpy.app.handlers.frame_change_pre.append(advance)
    bpy.app.handlers.load_post.append(_clear)
    bpy.app.handlers.undo_post.append(_clear)


def unregister():
    if advance in bpy.app.handlers.frame_change_pre:
        bpy.app.handlers.frame_change_pre.remove(advance)
    if _clear in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.remove(_clear)
    if _clear in bpy.app.handlers.undo_post:
        bpy.app.handlers.undo_post.remove(_clear)
    _sessions.clear()


def _clear(*args):
    _sessions.clear()
    _last_frames.clear()

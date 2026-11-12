import bpy
import numpy as np
from . import garment
from .piece_states import label, piece_indices
from .xpbd_live import evaluated_points


class StateOverlay:
    def __init__(self, editor):
        self.editor = editor
        self.groups_key = None
        self.groups = {}
        self.handle = bpy.types.SpaceView3D.draw_handler_add(self.draw, (), 'WINDOW', 'POST_PIXEL')

    def close(self):
        if self.handle is not None:
            bpy.types.SpaceView3D.draw_handler_remove(self.handle, 'WINDOW')
            self.handle = None

    def piece_groups(self, simulation):
        key = (simulation.name, len(simulation.data.vertices), simulation.get('dd_piece_ids'), simulation.get('dd_signature'))
        if key != self.groups_key:
            self.groups = {identity: np.array(indices, dtype=np.int32) for identity, indices in piece_indices(simulation).items()}
            self.groups_key = key
        return self.groups

    def draw(self):
        from mathutils import Vector
        from bpy_extras.view3d_utils import location_3d_to_region_2d
        import blf
        from .ui_system import ACCENT, TEXT_MUTED, SELECTION
        context = bpy.context
        editor = self.editor
        if editor._closed or editor.full or context.area != editor._area or context.region != editor._region or context.scene != editor._scene:
            return
        labelled = [(pattern, label(pattern)) for pattern in editor.session.document.patterns]
        labelled = [(pattern, text) for pattern, text in labelled if text]
        if not labelled:
            return
        simulations = garment.generated(context.scene, 'simulation')
        simulation = simulations[0] if simulations else None
        groups = self.piece_groups(simulation) if simulation else {}
        coordinates = evaluated_points(context, simulation) if simulation else None
        panels = {panel['dd_pattern_id']: panel for panel in garment.generated(context.scene, 'panel')}
        for pattern, text in labelled:
            indices = groups.get(pattern.id)
            if not pattern.deactivated and coordinates is not None and indices is not None and len(indices):
                center = simulation.matrix_world @ Vector(coordinates[indices].mean(axis=0).tolist())
            else:
                panel = panels.get(pattern.id)
                if panel is None or not panel.data.vertices:
                    continue
                flat = np.empty(len(panel.data.vertices) * 3, dtype=np.float32)
                panel.data.vertices.foreach_get('co', flat)
                center = panel.matrix_world @ Vector(flat.reshape(-1, 3).mean(axis=0).tolist())
            point = location_3d_to_region_2d(context.region, context.region_data, center)
            if point is None:
                continue
            blf.size(0, 12 * editor.canvas_layout.scale)
            blf.position(0, point.x, point.y, 0)
            blf.color(0, *(TEXT_MUTED if pattern.deactivated else ACCENT if pattern.frozen else SELECTION))
            blf.draw(0, pattern.name + ' · ' + text)

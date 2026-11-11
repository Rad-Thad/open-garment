import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT))

import bpy

import devil_designer
from devil_designer.core import Document


def check(condition, message):
    if not condition:
        raise AssertionError(message)
    print('PASS:', message, flush=True)


scratch = ROOT / 'scratch'
scratch.mkdir(exist_ok=True)

devil_designer.register()
scene = bpy.context.scene
scene.dd_solver = 'GPU'
scene.render.fps = 30
scene.frame_set(scene.frame_start)
check('FINISHED' in bpy.ops.devil_designer.demo(), 'demo garment created')

doc = Document.loads(scene.dd_document)
doc.sizes.append('L')
scene.dd_document = doc.dumps()

base = scratch / 'out.dxf'
check(bpy.ops.devil_designer.export_patterns(filepath=str(base), format='DXF') == {'FINISHED'}, 'DXF export runs')
check(base.exists(), 'DXF file written')

check(bpy.ops.devil_designer.export_patterns(filepath=str(base), format='PDF') == {'FINISHED'}, 'PDF export runs')
pdf_path = scratch / 'out.pdf'
check(pdf_path.exists(), 'PDF file written')

allbase = scratch / 'out2.dxf'
check(bpy.ops.devil_designer.export_patterns(filepath=str(allbase), format='DXF', all_sizes=True) == {'FINISHED'}, 'all-sizes export runs')
siblings = {path.name for path in scratch.iterdir() if path.name.startswith('out2')}
check(siblings == {'out2-Base.dxf', 'out2-L.dxf'}, 'all-sizes writes Base and L files')

dxf_bytes = base.read_bytes()
check(dxf_bytes.count(b'\r\n') == dxf_bytes.count(b'\n'), 'DXF uses CRLF line endings')

pdf_bytes = pdf_path.read_bytes()
startxref = int(pdf_bytes.split(b'startxref\n', 1)[1].split(b'\n', 1)[0])
check(startxref == pdf_bytes.index(b'\nxref\n') + 1, 'PDF startxref offset is correct')

missing = scratch / 'missing' / 'out.dxf'
result = {'CANCELLED'}
try:
    result = bpy.ops.devil_designer.export_patterns(filepath=str(missing), format='DXF')
except RuntimeError:
    pass
check(result == {'CANCELLED'}, 'export to missing directory is cancelled')
check(str(scene.get('dd_last_error', '')).startswith('Could not write'), 'export error reports Could not write')

devil_designer.unregister()
print('ALL PRODUCTION CHECKS PASSED', flush=True)

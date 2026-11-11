import tempfile
import unittest
from pathlib import Path

from devil_designer.core import Document, Session, area, clone_pattern
from devil_designer import production, pattern_export


class SeamAllowanceTests(unittest.TestCase):
    def setUp(self):
        self.session = Session()
        self.session.add([(0, 0), (.1, 0), (.1, .1), (0, .1)])
        self.pattern = self.session.document.patterns[0]

    def test_cut_outline_area(self):
        self.session.set_allowance(.01)
        cut = production.cut_outline(self.pattern)
        self.assertAlmostEqual(area(cut), .0144, delta=1e-6)
        self.session.set_edge_allowance(self.session.document.edge(self.pattern.id, 0), 0.0)
        cut = production.cut_outline(self.pattern)
        self.assertAlmostEqual(area(cut), .0132, delta=1e-6)

    def test_allowance_round_trip(self):
        self.session.set_allowance(.01)
        self.session.set_edge_allowance(self.session.document.edge(self.pattern.id, 0), .02)
        restored = Document.loads(self.session.document.dumps())
        piece = restored.patterns[0]
        self.assertAlmostEqual(piece.allowance, .01)
        self.assertAlmostEqual(piece.edge_allowances[piece.vertices[0].id], .02)
        self.assertEqual(restored.dumps(), self.session.document.dumps())

    def test_defaults_omit_allowance_fields(self):
        dumped = self.session.document.dumps()
        self.assertNotIn('allowance', dumped)
        self.assertNotIn('edge_allowances', dumped)


class GradingTests(unittest.TestCase):
    def setUp(self):
        self.session = Session()
        self.session.add([(0, 0), (.1, 0), (.1, .1), (0, .1)])
        self.pattern = self.session.document.patterns[0]

    def test_grade_corner_area(self):
        self.session.add_size('L')
        self.session.corner = 1
        self.session.set_grade('L', .01, 0.0)
        graded = self.session.document.graded('L')
        self.assertAlmostEqual(abs(area(graded.patterns[0].outline())), .0105, delta=1e-6)
        self.assertEqual(self.session.document.graded('').dumps(), self.session.document.dumps())

    def test_grade_round_trip(self):
        self.session.add_size('L')
        self.session.add_size('M')
        self.session.set_base_size('M')
        self.session.corner = 2
        self.session.set_grade('L', .01, .02)
        restored = Document.loads(self.session.document.dumps())
        self.assertEqual(restored.sizes, ['L', 'M'])
        self.assertEqual(restored.base_size, 'M')
        self.assertEqual(restored.patterns[0].vertices[2].grades['L'], (.01, .02))
        self.assertEqual(restored.dumps(), self.session.document.dumps())

    def test_ungraded_dumps_identical(self):
        dumped = self.session.document.dumps()
        self.assertNotIn('sizes', dumped)
        self.assertNotIn('base_size', dumped)
        self.assertNotIn('grades', dumped)

    def test_grade_preserves_seams(self):
        self.session.add([(.2, 0), (.3, 0), (.3, .1), (.2, .1)])
        first = self.session.document.patterns[0]
        second = self.session.document.patterns[1]
        self.session.sew(self.session.document.edge(first.id, 0), self.session.document.edge(second.id, 0))
        self.session.add_size('L')
        self.session.selected = first.id
        self.session.corner = 0
        self.session.set_grade('L', .01, 0.0)
        self.session.document.graded('L').validate_seams()


class EditGradeTests(unittest.TestCase):
    def setUp(self):
        self.session = Session()
        self.session.add([(0, 0), (.1, 0), (.1, .1), (0, .1)])
        self.pattern = self.session.document.patterns[0]
        self.session.add_size('L')
        self.session.corner = 1
        self.session.set_grade('L', .01, 0.0)
        self.session.corner = None
        self.session.selected = self.pattern.id

    def test_flip_horizontal_keeps_grade(self):
        self.session.flip_horizontal()
        graded = self.session.document.graded('L')
        self.assertAlmostEqual(abs(area(graded.patterns[0].outline())), .0105, delta=1e-6)

    def test_rotate_keeps_grade(self):
        self.session.rotate(90)
        graded = self.session.document.graded('L')
        self.assertAlmostEqual(abs(area(graded.patterns[0].outline())), .0105, delta=1e-6)

    def test_clone_reflection_keeps_grade(self):
        clone = clone_pattern(self.pattern, reflection=((0, 0), (0, 1)))
        doc = Document()
        doc.patterns = [clone]
        doc.sizes = ['L']
        graded = doc.graded('L')
        self.assertAlmostEqual(abs(area(graded.patterns[0].outline())), .0105, delta=1e-6)

    def test_split_edge_keeps_cut_area(self):
        self.session.set_edge_allowance(self.session.document.edge(self.pattern.id, 0), .02)
        before = area(production.cut_outline(self.pattern))
        self.session.document.split_edge(self.session.document.edge(self.pattern.id, 0), .5)
        after = area(production.cut_outline(self.pattern))
        self.assertAlmostEqual(after, before, delta=1e-9)

    def test_duplicate_keeps_cut_area(self):
        self.session.duplicate()
        source = self.session.document.patterns[0]
        copy = self.session.document.patterns[-1]
        self.assertAlmostEqual(area(production.cut_outline(copy)), area(production.cut_outline(source)), delta=1e-9)


class MeasuredFabricTests(unittest.TestCase):
    def setUp(self):
        self.session = Session()
        self.session.add([(0, 0), (.1, 0), (.1, .1), (0, .1)])

    def test_valid_fabric_measured(self):
        from devil_designer.presets import valid_fabric, valid_measured, measured_to_preset
        measured = dict(weight_gsm=150, stretch_warp=100, stretch_weft=90, stretch_bias=80,
                        bend_warp=2.0, bend_weft=2.5, bend_bias=3.0)
        self.assertTrue(valid_measured(measured))
        self.assertTrue(valid_fabric(measured_to_preset(measured)))
        invalid = dict(measured)
        invalid['weight_gsm'] = 0
        self.assertFalse(valid_measured(invalid))
        self.assertFalse(valid_fabric(dict(structural=1, bending=1, density=1, measured=invalid)))
        self.assertFalse(valid_fabric(dict(structural=1, bending=1, density=1, measured='x')))

    def test_measured_round_trip(self):
        from devil_designer.presets import measured_to_preset
        measured = dict(weight_gsm=150, stretch_warp=100, stretch_weft=90, stretch_bias=80,
                        bend_warp=2.0, bend_weft=2.5, bend_bias=3.0)
        self.session.set_custom_preset('MeasuredCloth', measured_to_preset(measured))
        self.session.set_document_preset('MeasuredCloth')
        restored = Document.loads(self.session.document.dumps())
        self.assertEqual(restored.custom_presets['MeasuredCloth']['measured'], measured)
        self.assertEqual(restored.dumps(), self.session.document.dumps())


class ExportTests(unittest.TestCase):
    def setUp(self):
        self.session = Session()
        self.session.add([(0, 0), (.1, 0), (.1, .1), (0, .1)])
        self.pattern = self.session.document.patterns[0]

    def test_export_dxf(self):
        self.session.set_allowance(.01)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'pattern.dxf'
            pattern_export.write_dxf(path, self.session.document, 'Style', '')
            text = path.read_text(encoding='ascii')
        self.assertEqual(text.count('\nBLOCK\n'), 1)
        self.assertIn('POLYLINE\n8\n1\n66\n1\n70\n1\n', text)
        perimeters = _dxf_polyline_perimeters(text)
        self.assertTrue(any(abs(value - 480) < .5 for value in perimeters.get('1', [])), f'cut perimeter {perimeters.get("1")}')
        self.assertTrue(any(abs(value - 400) < .5 for value in perimeters.get('14', [])), f'stitch perimeter {perimeters.get("14")}')
        self.assertIn('Units: METRIC', text)

    def test_export_pdf(self):
        large = Session()
        large.add([(0, 0), (.5, 0), (.5, .7), (0, .7)])
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'pattern.pdf'
            pattern_export.write_pdf(path, large.document, '', 'A4')
            text = path.read_text(encoding='latin-1')
        self.assertTrue(text.startswith('%PDF-1.4'))
        self.assertEqual(text.count('/Type /Page '), 9)
        self.assertIn('/Helvetica', text)

    def test_pdf_true_scale(self):
        session = Session()
        session.add([(0, 0), (.1, 0), (.1, .1), (0, .1)])
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'pattern.pdf'
            pattern_export.write_pdf(path, session.document, '', 'A4')
            text = path.read_text(encoding='latin-1')
        stream = text.split('stream\n', 1)[1].split('\nendstream', 1)[0]
        tokens = stream.split()
        index = tokens.index('m')
        points = [(float(tokens[index - 2]), float(tokens[index - 1]))]
        for position in range(index + 1, len(tokens)):
            if tokens[position] == 'l':
                points.append((float(tokens[position - 2]), float(tokens[position - 1])))
            elif tokens[position] == 'h':
                break
        xs = [point[0] for point in points]
        ys = [point[1] for point in points]
        self.assertAlmostEqual(min(xs), 28.346, delta=0.01)
        self.assertAlmostEqual(min(ys), 28.346, delta=0.01)
        self.assertAlmostEqual(max(xs) - min(xs), 283.465, delta=0.01)
        self.assertAlmostEqual(max(ys) - min(ys), 283.465, delta=0.01)

    def test_export_accepts_string_path(self):
        with tempfile.TemporaryDirectory() as directory:
            dxf = Path(directory) / 'pattern.dxf'
            pdf = Path(directory) / 'pattern.pdf'
            pattern_export.write_dxf(str(dxf), self.session.document, 'Style', '')
            pattern_export.write_pdf(str(pdf), self.session.document, '', 'A4')
            self.assertTrue(dxf.exists())
            self.assertTrue(pdf.exists())

    def test_export_non_ascii_name(self):
        session = Session()
        session.add([(0, 0), (.1, 0), (.1, .1), (0, .1)])
        session.document.patterns[0].name = 'Ärmel'
        with tempfile.TemporaryDirectory() as directory:
            dxf = Path(directory) / 'pattern.dxf'
            pdf = Path(directory) / 'pattern.pdf'
            pattern_export.write_dxf(dxf, session.document, 'Style', '')
            pattern_export.write_pdf(pdf, session.document, '', 'A4')
            self.assertTrue(dxf.read_bytes())
            self.assertTrue(pdf.read_bytes())


def _dxf_entities(text):
    codes = []
    lines = text.splitlines()
    for index in range(0, len(lines) - 1, 2):
        codes.append((int(lines[index].strip()), lines[index + 1].rstrip()))
    return codes


def _dxf_polyline_perimeters(text):
    perimeters = {}
    layer = None
    points = None
    x = None
    y = None
    for code, value in _dxf_entities(text):
        if code == 0:
            if value == 'POLYLINE':
                points = []
                x = y = None
            elif value == 'VERTEX':
                if points is not None and x is not None:
                    points.append((x, y))
                x = y = None
            elif value == 'SEQEND':
                if points is not None and layer is not None:
                    if x is not None:
                        points.append((x, y))
                    perimeter = sum(((points[i][0] - points[i - 1][0]) ** 2 + (points[i][1] - points[i - 1][1]) ** 2) ** .5 for i in range(len(points)))
                    perimeters.setdefault(layer, []).append(perimeter)
                points = None
                x = y = None
        elif code == 8:
            layer = value
        elif code == 10 and points is not None:
            x = float(value)
        elif code == 20 and points is not None:
            y = float(value)
    return perimeters


if __name__ == '__main__':
    unittest.main()

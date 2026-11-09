import unittest

from devil_designer.core import Document, Session, area
from devil_designer import production


class SeamAllowanceTests(unittest.TestCase):
    def setUp(self):
        self.session = Session()
        self.session.add([(0, 0), (.1, 0), (.1, .1), (0, .1)])
        self.pattern = self.session.document.patterns[0]

    def test_cut_outline_area(self):
        self.session.set_allowance(.01)
        cut = production.cut_outline(self.session.document, self.pattern)
        self.assertAlmostEqual(area(cut), .0144, delta=1e-6)
        self.session.set_edge_allowance(self.session.document.edge(self.pattern.id, 0), 0.0)
        cut = production.cut_outline(self.session.document, self.pattern)
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
        self.assertAlmostEqual(area(graded.patterns[0].outline()), .0105, delta=1e-6)
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


if __name__ == '__main__':
    unittest.main()

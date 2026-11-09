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


if __name__ == '__main__':
    unittest.main()

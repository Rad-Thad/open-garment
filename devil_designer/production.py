import math

from . import curves
from .core import area


def segment_allowance(pattern, index):
    overrides = getattr(pattern, 'edge_allowances', None)
    if overrides:
        start = pattern.vertices[index].id
        if start in overrides:
            return overrides[start]
    return getattr(pattern, 'allowance', 0.0)


def _tangent(control, t):
    a, b, c, d = control
    dx = 3 * ((1 - t) ** 2 * (b[0] - a[0]) + 2 * (1 - t) * t * (c[0] - b[0]) + t * t * (d[0] - c[0]))
    dy = 3 * ((1 - t) ** 2 * (b[1] - a[1]) + 2 * (1 - t) * t * (c[1] - b[1]) + t * t * (d[1] - c[1]))
    length = math.hypot(dx, dy)
    if length < 1e-12:
        return (0.0, 0.0)
    return (dx / length, dy / length)


def _segment(document, pattern, index, sign):
    edge = document.edge(pattern.id, index)
    control = document.edge_curve(edge)
    a = pattern.vertices[index]
    b = pattern.vertices[(index + 1) % len(pattern.vertices)]
    if a.outgoing is None and b.incoming is None:
        ts = (0.0, 1.0)
    else:
        ts = tuple(k / 24 for k in range(24))
    samples = [curves.evaluate(control, t) for t in ts]
    tangents = [_tangent(control, t) for t in ts]
    distance = segment_allowance(pattern, index)
    points = [(x + sign * ty * distance, y - sign * tx * distance) for (x, y), (tx, ty) in zip(samples, tangents)]
    return dict(points=points, start=tangents[0], end=tangents[-1], allowance=distance)


def _join(previous, current, vertex):
    end = previous['points'][-1]
    start = current['points'][0]
    ti = previous['end']
    tj = current['start']
    determinant = ti[0] * tj[1] - ti[1] * tj[0]
    if abs(determinant) < 1e-9:
        return [end, start]
    dx = start[0] - end[0]
    dy = start[1] - end[1]
    step = (dx * tj[1] - dy * tj[0]) / determinant
    miter = (end[0] + step * ti[0], end[1] + step * ti[1])
    limit = 4 * max(previous['allowance'], current['allowance'])
    if math.dist(miter, (vertex.x, vertex.y)) > limit:
        return [end, start]
    return [miter]


def cut_outline(document, pattern):
    count = len(pattern.vertices)
    allowances = [segment_allowance(pattern, index) for index in range(count)]
    if all(value == 0.0 for value in allowances):
        return pattern.outline()
    sign = 1 if area(pattern.outline()) > 0 else -1
    segments = [_segment(document, pattern, index, sign) for index in range(count)]
    result = []
    for index in range(count):
        result.extend(_join(segments[(index - 1) % count], segments[index], pattern.vertices[index]))
        result.extend(segments[index]['points'][1:-1])
    return result

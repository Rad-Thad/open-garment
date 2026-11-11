import math

from . import production

LAYERS = ('1', '2', '3', '7', '8', '11', '14', '15')
MM = 1000.0
PDF_SCALE = 72.0 / 25.4
PAPER = {'A4': (210.0, 297.0), 'LETTER': (215.9, 279.4)}
MARGIN = 10.0


def pieces(document):
    result = []
    for pattern in document.patterns:
        outline = pattern.outline()
        cut = production.cut_outline(pattern)
        internal = [(path.outline(), path.closed) for path in pattern.paths if path.role == 'INTERNAL']
        cutouts = [(path.outline(), path.closed) for path in pattern.paths if path.role in ('HOLE', 'SLIT')]
        corners = [(vertex.x, vertex.y) for vertex in pattern.vertices if getattr(vertex, 'point_type', 'CORNER') == 'CORNER']
        xs = [point[0] for point in outline]
        ys = [point[1] for point in outline]
        center = ((min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2)
        height = max(ys) - min(ys)
        grain = ((center[0], center[1] - height / 4), (center[0], center[1] + height / 4))
        result.append(dict(name=pattern.name, cut=cut, stitch=outline, internal=internal, cutouts=cutouts,
                           corners=corners, grain=grain, fabric=getattr(pattern, 'preset', 'COTTON')))
    return result


def _f(value):
    text = f'{value:.6f}'.rstrip('0').rstrip('.')
    return text or '0'


def _sanitize(name):
    cleaned = ''.join(character if character.isalnum() else '_' for character in str(name))
    return cleaned or 'Piece'


def _emit(lines, code, value):
    lines.append(str(code))
    lines.append(str(value))


def _polyline(lines, layer, points, closed):
    _emit(lines, 0, 'POLYLINE')
    _emit(lines, 8, layer)
    _emit(lines, 66, 1)
    _emit(lines, 70, 1 if closed else 0)
    for x, y in points:
        _emit(lines, 0, 'VERTEX')
        _emit(lines, 8, layer)
        _emit(lines, 10, _f(x * MM))
        _emit(lines, 20, _f(y * MM))
    _emit(lines, 0, 'SEQEND')


def _point(lines, layer, position):
    _emit(lines, 0, 'POINT')
    _emit(lines, 8, layer)
    _emit(lines, 10, _f(position[0] * MM))
    _emit(lines, 20, _f(position[1] * MM))
    _emit(lines, 30, '0')


def _line(lines, layer, start, end):
    _emit(lines, 0, 'LINE')
    _emit(lines, 8, layer)
    _emit(lines, 10, _f(start[0] * MM))
    _emit(lines, 20, _f(start[1] * MM))
    _emit(lines, 30, '0')
    _emit(lines, 11, _f(end[0] * MM))
    _emit(lines, 21, _f(end[1] * MM))
    _emit(lines, 31, '0')


def _text(lines, layer, text, position, height):
    _emit(lines, 0, 'TEXT')
    _emit(lines, 8, layer)
    _emit(lines, 10, _f(position[0] * MM))
    _emit(lines, 20, _f(position[1] * MM))
    _emit(lines, 40, _f(height * MM))
    _emit(lines, 1, text)


def _differs(first, second):
    if len(first) != len(second):
        return True
    return any(abs(a[0] - b[0]) > 1e-9 or abs(a[1] - b[1]) > 1e-9 for a, b in zip(first, second))


def write_dxf(path, document, style_name, size_name):
    all_pieces = pieces(document)
    if not all_pieces:
        raise ValueError('Draw at least one closed pattern first.')
    lines = []
    _emit(lines, 0, 'SECTION')
    _emit(lines, 2, 'HEADER')
    _emit(lines, 9, '$ACADVER')
    _emit(lines, 1, 'AC1009')
    _emit(lines, 0, 'ENDSEC')
    _emit(lines, 0, 'SECTION')
    _emit(lines, 2, 'TABLES')
    _emit(lines, 0, 'TABLE')
    _emit(lines, 2, 'LAYER')
    _emit(lines, 70, len(LAYERS))
    for layer in LAYERS:
        _emit(lines, 0, 'LAYER')
        _emit(lines, 2, layer)
        _emit(lines, 70, 0)
        _emit(lines, 62, 7)
        _emit(lines, 6, 'CONTINUOUS')
    _emit(lines, 0, 'ENDTAB')
    _emit(lines, 0, 'ENDSEC')
    _emit(lines, 0, 'SECTION')
    _emit(lines, 2, 'BLOCKS')
    block_names = []
    for index, piece in enumerate(all_pieces):
        block_name = _sanitize(piece['name']) + str(index)
        block_names.append(block_name)
        _emit(lines, 0, 'BLOCK')
        _emit(lines, 8, '1')
        _emit(lines, 2, block_name)
        _emit(lines, 70, 0)
        _emit(lines, 10, '0')
        _emit(lines, 20, '0')
        _emit(lines, 30, '0')
        _polyline(lines, '1', piece['cut'], True)
        if _differs(piece['stitch'], piece['cut']):
            _polyline(lines, '14', piece['stitch'], True)
        for internal, closed in piece['internal']:
            _polyline(lines, '8', internal, closed)
        for cutout, closed in piece['cutouts']:
            _polyline(lines, '11', cutout, closed)
        corner_keys = {_f(x * MM) + ',' + _f(y * MM) for x, y in piece['corners']}
        for corner in piece['corners']:
            _point(lines, '2', corner)
        for sample in piece['stitch']:
            if _f(sample[0] * MM) + ',' + _f(sample[1] * MM) not in corner_keys:
                _point(lines, '3', sample)
        _line(lines, '7', piece['grain'][0], piece['grain'][1])
        ys = [point[1] for point in piece['cut']]
        label_y = min(ys) - .04
        _text(lines, '1', f'Piece Name: {piece["name"]}', (0, label_y), .01)
        _text(lines, '1', f'Size: {size_name or "Base"}', (0, label_y - .015), .01)
        _text(lines, '1', 'Quantity: 1', (0, label_y - .03), .01)
        _text(lines, '1', f'Material: {piece["fabric"]}', (0, label_y - .045), .01)
        _emit(lines, 0, 'ENDBLK')
    _emit(lines, 0, 'ENDSEC')
    _emit(lines, 0, 'SECTION')
    _emit(lines, 2, 'ENTITIES')
    for block_name in block_names:
        _emit(lines, 0, 'INSERT')
        _emit(lines, 8, '0')
        _emit(lines, 2, block_name)
        _emit(lines, 10, '0')
        _emit(lines, 20, '0')
        _emit(lines, 30, '0')
    _text(lines, '1', f'Style Name: {style_name}', (0, -.1), .02)
    _text(lines, '1', f'Creation Date: {_today()}', (0, -.12), .02)
    _text(lines, '1', 'Author: Open Garment', (0, -.14), .02)
    _text(lines, '1', f'Sample Size: {_base_size(document)}', (0, -.16), .02)
    _text(lines, '1', 'Units: METRIC', (0, -.18), .02)
    _emit(lines, 0, 'ENDSEC')
    _emit(lines, 0, 'EOF')
    data = ('\r\n'.join(lines) + '\r\n').encode('ascii', 'replace')
    with open(path, 'wb') as handle:
        handle.write(data)


def _base_size(document):
    return getattr(document, 'base_size', '') or 'Base'


def _today():
    import datetime
    return datetime.date.today().isoformat()


def write_pdf(path, document, size_name, paper):
    all_pieces = pieces(document)
    if not all_pieces:
        raise ValueError('Draw at least one closed pattern first.')
    width, height = PAPER[paper]
    page_w = width - 2 * MARGIN
    page_h = height - 2 * MARGIN
    min_x = min(point[0] for piece in all_pieces for point in piece['cut'])
    min_y = min(point[1] for piece in all_pieces for point in piece['cut'])
    max_x = max(point[0] for piece in all_pieces for point in piece['cut'])
    max_y = max(point[1] for piece in all_pieces for point in piece['cut'])
    cols = max(1, math.ceil((max_x - min_x) * MM / page_w))
    rows = max(1, math.ceil((max_y - min_y) * MM / page_h))
    header = '%PDF-1.4\n'
    catalog = '1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n'
    kids = []
    page_objects = []
    number = 3
    for row in range(rows):
        for col in range(cols):
            content = _pdf_page(all_pieces, min_x, min_y, size_name, col, row, page_w, page_h, width, height)
            kids.append(f'{number} 0 R')
            page_objects.append(f'{number} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {_f(width * PDF_SCALE)} {_f(height * PDF_SCALE)}] /Resources << /Font << /F1 {number + 1} 0 R >> >> /Contents {number + 2} 0 R >>\nendobj\n')
            page_objects.append(f'{number + 1} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n')
            page_objects.append(f'{number + 2} 0 obj\n<< /Length {len(content.encode("latin-1"))} >>\nstream\n{content}\nendstream\nendobj\n')
            number += 3
    page_tree = f'2 0 obj\n<< /Type /Pages /Kids [{" ".join(kids)}] /Count {len(kids)} >>\nendobj\n'
    objects = [catalog, page_tree] + page_objects
    body = header + ''.join(objects)
    cursor = len(header.encode('latin-1'))
    offsets = [0]
    for obj in objects:
        offsets.append(cursor)
        cursor += len(obj.encode('latin-1'))
    xref = 'xref\n0 ' + str(len(offsets)) + '\n0000000000 65535 f \n'
    xref += ''.join(f'{offset:010d} 00000 n \n' for offset in offsets[1:])
    trailer = f'trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{cursor}\n%%EOF\n'
    data = (body + xref + trailer).encode('latin-1')
    with open(path, 'wb') as handle:
        handle.write(data)


def _pdf_page(all_pieces, min_x, min_y, size_name, col, row, page_w, page_h, width, height):
    def place(point):
        return (((point[0] - min_x) * MM - col * page_w + MARGIN) * PDF_SCALE,
                ((point[1] - min_y) * MM - row * page_h + MARGIN) * PDF_SCALE)
    operations = []
    for piece in all_pieces:
        cut = [place(point) for point in piece['cut']]
        stitch = [place(point) for point in piece['stitch']]
        _pdf_polyline(operations, cut, '0.5', None, True)
        _pdf_polyline(operations, stitch, '0.3', '[3 3] 0 d', True)
        for internal, closed in piece['internal']:
            _pdf_polyline(operations, [place(point) for point in internal], '0.3', None, closed)
        for cutout, closed in piece['cutouts']:
            _pdf_polyline(operations, [place(point) for point in cutout], '0.3', None, closed)
        grain = piece['grain']
        _pdf_segment(operations, place(grain[0]), place(grain[1]), '0.3', None)
        _pdf_arrow(operations, (place(grain[0]), place(grain[1])))
        xs = [point[0] for point in piece['cut']]
        ys = [point[1] for point in piece['cut']]
        label = f"{piece['name']}  {size_name or 'Base'}"
        cx, cy = place(((min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2))
        operations.append('BT /F1 8 Tf')
        operations.append(f'{_f(cx)} {_f(cy)} Td')
        operations.append(f'({_pdf_escape(label)}) Tj ET')
    half = 4 * PDF_SCALE
    for px in (MARGIN * PDF_SCALE, (width - MARGIN) * PDF_SCALE):
        for py in (MARGIN * PDF_SCALE, (height - MARGIN) * PDF_SCALE):
            _cross(operations, px, py, half)
    operations.append('BT /F1 8 Tf')
    operations.append(f'{_f(MARGIN * PDF_SCALE)} {_f((height - MARGIN) * PDF_SCALE + 12)} Td')
    operations.append(f'({row}-{col}) Tj ET')
    if col == 0 and row == 0:
        x0 = (width - MARGIN) * PDF_SCALE - 50 * PDF_SCALE
        y0 = (height - MARGIN) * PDF_SCALE - 50 * PDF_SCALE
        operations.append('0.5 w')
        operations.append(f'{_f(x0)} {_f(y0)} {_f(50 * PDF_SCALE)} {_f(50 * PDF_SCALE)} re S')
    return '\n'.join(operations)


def _cross(operations, x, y, half):
    operations.append('0.3 w')
    operations.append(f'{_f(x - half)} {_f(y)} m {_f(x + half)} {_f(y)} l S')
    operations.append(f'{_f(x)} {_f(y - half)} m {_f(x)} {_f(y + half)} l S')


def _pdf_polyline(operations, points, width, dash, closed):
    if not points:
        return
    _pdf_start(operations, width, dash)
    x0, y0 = points[0]
    operations.append(f'{_f(x0)} {_f(y0)} m')
    for x, y in points[1:]:
        operations.append(f'{_f(x)} {_f(y)} l')
    if closed:
        operations.append('h')
    operations.append('S')


def _pdf_segment(operations, start, end, width, dash):
    _pdf_start(operations, width, dash)
    operations.append(f'{_f(start[0])} {_f(start[1])} m')
    operations.append(f'{_f(end[0])} {_f(end[1])} l')
    operations.append('S')


def _pdf_arrow(operations, grain):
    start, end = grain
    dx = end[0] - start[0]
    dy = end[1] - start[1]
    length = math.hypot(dx, dy) or 1.0
    ux, uy = dx / length, dy / length
    tip = (end[0], end[1])
    back = (tip[0] - ux * 6, tip[1] - uy * 6)
    left = (back[0] - uy * 3, back[1] + ux * 3)
    right = (back[0] + uy * 3, back[1] - ux * 3)
    operations.append('0.3 w')
    operations.append(f'{_f(tip[0])} {_f(tip[1])} m {_f(left[0])} {_f(left[1])} l {_f(right[0])} {_f(right[1])} l h')
    operations.append('S')


def _pdf_start(operations, width, dash):
    operations.append(f'{width} w')
    operations.append('[] 0 d' if dash is None else dash)


def _pdf_escape(text):
    return str(text).encode('latin-1', 'replace').decode('latin-1').replace('\\', '\\\\').replace('(', '\\(').replace(')', '\\)')

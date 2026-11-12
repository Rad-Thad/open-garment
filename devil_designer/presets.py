import math

MASS_SCALE = 1000.0
STRETCH_COMPLIANCE = 1.0e-8
HINGE_SCALE = 3.0


PRESETS = {
    'SILK': dict(structural=12.3, bending=0.55, density=0.07),
    'KNIT': dict(structural=7.1, bending=0.95, density=0.17),
    'COTTON': dict(structural=32.0, bending=2.2, density=0.14),
    'LINEN': dict(structural=65.0, bending=6.0, density=0.19),
    'DENIM': dict(structural=120.0, bending=68.0, density=0.40),
    'LEATHER': dict(structural=180.0, bending=115.0, density=0.60),
}


FABRIC_DEFAULTS = dict(weft=1.0, bias=1.0, shrink_warp=0.0, shrink_weft=0.0)
REQUIRED = {'structural', 'bending', 'density'}
SHRINK_LIMIT = .3
AIR_DRAG_DENSITY = .14
MEASURED_KEYS = {'weight_gsm', 'stretch_warp', 'stretch_weft', 'stretch_bias', 'bend_warp', 'bend_weft', 'bend_bias'}


def valid_measured(measured):
    if not isinstance(measured, dict) or set(measured) != MEASURED_KEYS:
        return False
    return all(type(value) in (int, float) and math.isfinite(value) and value > 0 for value in measured.values())


def valid_fabric(values):
    if not isinstance(values, dict):
        return False
    keys = set(values)
    if 'measured' in keys:
        keys.remove('measured')
        if not valid_measured(values['measured']):
            return False
    if not REQUIRED <= keys <= REQUIRED | set(FABRIC_DEFAULTS):
        return False
    for key, value in values.items():
        if key == 'measured':
            continue
        if type(value) not in (int, float) or not math.isfinite(value):
            return False
        if key.startswith('shrink_') and abs(value) > SHRINK_LIMIT or not key.startswith('shrink_') and value <= 0:
            return False
    return True


def air_damping(density):
    return max(1.0, AIR_DRAG_DENSITY / density)


def measured_to_preset(measured):
    density = measured['weight_gsm'] / 1000
    structural = measured['stretch_warp'] * STRETCH_COMPLIANCE * 32 * MASS_SCALE
    bending = min(PRESETS.values(), key=lambda preset: abs(preset['density'] - density))['bending']
    return dict(structural=structural, bending=bending, density=density, measured=measured)


def resolve_preset(name, custom_presets=None):
    if custom_presets and name in custom_presets:
        return {**FABRIC_DEFAULTS, **custom_presets[name]}
    return {**FABRIC_DEFAULTS, **PRESETS.get(name, PRESETS['COTTON'])}

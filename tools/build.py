import argparse
import platform
import re
import sys
import tomllib
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED

root = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(root))
from tools.workspace import extension_archive
from tools.check_comments import check_repository
from tools.check_dead_code import check_dead_code, check_import_integrity


EXCLUDE_PATTERNS = ('assets/icons/*.png', 'assets/icons/*.svg')
HOST_PLATFORMS = {'Darwin': 'macos-arm64', 'Windows': 'windows-x64', 'Linux': 'linux-x64'}


def manifest():
    return tomllib.loads((root / 'devil_designer' / 'blender_manifest.toml').read_text(encoding='utf-8'))


def present_wheels():
    return sorted((root / 'devil_designer' / 'wheels').glob('garment_solver-*.whl'))


def check_wheels(local):
    wheels = manifest().get('wheels', [])
    if local:
        if not present_wheels():
            raise SystemExit('Missing solver wheels: run tools/build_solver.py or download CI wheels.')
        return
    for wheel in wheels:
        if not (root / 'devil_designer' / wheel.lstrip('./')).is_file():
            raise SystemExit('Missing solver wheels: run tools/build_solver.py or download CI wheels.')


def local_manifest_text():
    text = (root / 'devil_designer' / 'blender_manifest.toml').read_text(encoding='utf-8')
    host = HOST_PLATFORMS.get(platform.system(), 'linux-x64')
    text = re.sub(r'platforms = \[[^\]]*\]', f'platforms = ["{host}"]', text, count=1)
    entries = ',\n'.join(f'    "./wheels/{wheel.name}"' for wheel in present_wheels())
    text = re.sub(r'wheels = \[[^\]]*\]', f'wheels = [\n{entries},\n]', text, count=1, flags=re.DOTALL)
    return text


def build_extension(local=False):
    check_repository(root)
    check_import_integrity(root)
    check_dead_code(root)
    check_wheels(local)
    source = root / 'devil_designer'
    output = extension_archive()
    output.parent.mkdir(exist_ok=True)
    suffixes = {'.py', '.toml', '.md', '.txt', '.json', '.svg', '.png', '.whl'}
    with ZipFile(output, 'w', ZIP_DEFLATED) as archive:
        for file in sorted(source.rglob('*')):
            rel = file.relative_to(source)
            if file.is_file() and '__pycache__' not in file.parts and not any(rel.match(pattern) for pattern in EXCLUDE_PATTERNS) and file.suffix in suffixes:
                if local and rel.name == 'blender_manifest.toml':
                    archive.writestr(rel.as_posix(), local_manifest_text())
                else:
                    archive.write(file, rel.as_posix())
        for name in ('README.md', 'LICENSE', 'SPEC.md'):
            archive.write(root / name, name)
    print(output)
    return output


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--local', action='store_true')
    args = parser.parse_args()
    build_extension(args.local)

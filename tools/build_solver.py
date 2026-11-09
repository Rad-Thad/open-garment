import os
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WHEELS = ROOT / 'devil_designer' / 'wheels'
SITE = ROOT / 'scratch' / 'solver_site'


def main():
    WHEELS.mkdir(parents=True, exist_ok=True)
    for wheel in WHEELS.glob('garment_solver-*.whl'):
        wheel.unlink()
    env = dict(os.environ)
    cargo_bin = Path.home() / '.cargo' / 'bin'
    if cargo_bin.is_dir():
        env['PATH'] = str(cargo_bin) + os.pathsep + env.get('PATH', '')
    subprocess.run(
        [sys.executable, '-m', 'maturin', 'build', '--release',
         '--manifest-path', str(ROOT / 'solver' / 'Cargo.toml'), '--out', str(WHEELS)],
        check=True, env=env)
    wheels = list(WHEELS.glob('garment_solver-*.whl'))
    if not wheels:
        raise SystemExit('maturin produced no wheel.')
    if SITE.exists():
        shutil.rmtree(SITE)
    SITE.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(wheels[0]) as archive:
        archive.extractall(SITE)


if __name__ == '__main__':
    main()

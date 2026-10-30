from pathlib import Path
import subprocess

from PIL import Image, ImageChops

root = Path(__file__).resolve().parent.parent
baseline_dir = root / "ln_drawer" / "examples"
regression_dir = root / "target" / "regression"
actual_dir = regression_dir / "actual"
diff_dir = actual_dir / "diff"

subprocess.run([root / "scripts" / "gen-regression.sh", actual_dir])

for stale in diff_dir.glob("*.png"):
    stale.unlink()
diff_dir.mkdir(parents=True, exist_ok=True)

for baseline_path in sorted(baseline_dir.glob("*.png")):
    name = baseline_path.name
    baseline = Image.open(baseline_path).convert("RGBA")
    actual = Image.open(actual_dir / name).convert("RGBA")

    if baseline.size != actual.size:
        print(f"{name}: size differs")
        continue

    diff = ImageChops.difference(baseline, actual)
    bands = diff.split()
    mask = bands[0]
    for band in bands[1:]:
        mask = ImageChops.lighter(mask, band)
    mask = mask.point(lambda value: 255 if value else 0)

    pixels = sum(mask.histogram()[1:])
    if not pixels:
        print(f"{name}: match")
        continue

    dim = Image.eval(baseline.convert("RGB"), lambda value: value // 4)
    highlight = Image.new("RGB", baseline.size, (255, 0, 0))
    Image.composite(highlight, dim, mask).save(diff_dir / name)
    print(f"{name}: {pixels} pixel(s) differ")

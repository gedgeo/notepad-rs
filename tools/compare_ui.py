"""Normalize a generated design and native client screenshot for visual QA."""
import argparse
from pathlib import Path
from PIL import Image, ImageDraw

parser = argparse.ArgumentParser()
parser.add_argument("reference", type=Path)
parser.add_argument("implementation", type=Path)
parser.add_argument("output", type=Path)
parser.add_argument("--crop", type=int, default=44, help="Reference Windows title-bar height")
parser.add_argument("--region", help="Optional normalized left,top,right,bottom crop")
args = parser.parse_args()
reference = Image.open(args.reference).convert("RGB")
reference = reference.crop((0, args.crop, reference.width, reference.height))
implementation = Image.open(args.implementation).convert("RGB")
reference = reference.resize(implementation.size, Image.Resampling.LANCZOS)
if args.region:
    box = tuple(map(int, args.region.split(",")))
    reference, implementation = reference.crop(box), implementation.crop(box)
result = Image.new("RGB", (reference.width * 2, reference.height + 28), "#e5e7eb")
draw = ImageDraw.Draw(result)
draw.text((12, 8), "REFERENCE", fill="#172033")
draw.text((reference.width + 12, 8), "APPLICATION NATIVE", fill="#172033")
result.paste(reference, (0, 28))
result.paste(implementation, (reference.width, 28))
args.output.parent.mkdir(parents=True, exist_ok=True)
result.save(args.output)

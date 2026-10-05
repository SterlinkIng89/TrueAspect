"""Automated black-bar cropping tool for ultrawide Steam screenshots.

Detects letterboxing/pillarboxing and crops images to a target aspect ratio
(default: 16:9) without overwriting original files.
"""
from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path

from PIL import Image

SUPPORTED_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp"}
DEFAULT_BLACK_THRESHOLD = 20  # Max pixel luminance (0-255) to consider black


def is_black_region(image: Image.Image, box: tuple[int, int, int, int], threshold: int = DEFAULT_BLACK_THRESHOLD) -> bool:
    """Check if the specified bounding box in the image contains only black pixels."""
    if box[2] <= box[0] or box[3] <= box[1]:
        return True
    
    region = image.crop(box).convert("L")
    _, max_val = region.getextrema()
    return max_val <= threshold


def calculate_target_box(width: int, height: int, target_ratio: float) -> tuple[int, int, int, int]:
    """Calculate centered bounding box matching the desired aspect ratio."""
    current_ratio = width / height
    if current_ratio > target_ratio:
        # Pillarbox (black bars on left and right)
        target_width = round(height * target_ratio)
        offset_x = (width - target_width) // 2
        return (offset_x, 0, offset_x + target_width, height)
    
    # Letterbox (black bars on top and bottom)
    target_height = round(width / target_ratio)
    offset_y = (height - target_height) // 2
    return (0, offset_y, width, offset_y + target_height)


def process_screenshot(
    image_path: Path,
    output_dir: Path,
    target_ratio: float,
    threshold: int = DEFAULT_BLACK_THRESHOLD,
    quality: int = 95,
    dry_run: bool = False,
) -> str:
    """Process a single screenshot. Crops if bars are black, otherwise copies intact."""
    output_path = output_dir / image_path.name

    with Image.open(image_path) as img:
        width, height = img.size
        box = calculate_target_box(width, height, target_ratio)

        if box == (0, 0, width, height):
            status = "Already target aspect ratio; skipped crop"
            cropped = None
        else:
            # Check the candidate discarded margins
            left_bar = (0, 0, box[0], height)
            right_bar = (box[2], 0, width, height)
            top_bar = (0, 0, width, box[1])
            bottom_bar = (0, box[3], width, height)

            bars = [left_bar, right_bar, top_bar, bottom_bar]
            if all(is_black_region(img, bar, threshold) for bar in bars):
                new_width = box[2] - box[0]
                new_height = box[3] - box[1]
                status = f"Cropped {width}x{height} -> {new_width}x{new_height}"
                cropped = img.crop(box)
            else:
                status = "Non-black margins detected; copied without cropping"
                cropped = None

        if dry_run:
            return status

        if cropped is not None:
            save_kwargs = {}
            if image_path.suffix.lower() in {".jpg", ".jpeg"}:
                save_kwargs = {"quality": quality, "subsampling": 0}
            cropped.save(output_path, **save_kwargs)

    if not dry_run and cropped is None:
        shutil.copy2(image_path, output_path)

    return status


def parse_aspect_ratio(ratio_str: str) -> float:
    """Parse aspect ratio string like '16:9' or '21:9' into a float value."""
    try:
        parts = ratio_str.split(":")
        if len(parts) == 2:
            return float(parts[0]) / float(parts[1])
        return float(ratio_str)
    except ValueError as exc:
        raise argparse.ArgumentTypeError(f"Invalid ratio format: '{ratio_str}'. Use e.g. 16:9 or 21:9.") from exc


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Automatically detect and crop black bars from ultrawide Steam screenshots."
    )
    parser.add_argument(
        "source",
        nargs="?",
        type=Path,
        default=None,
        help="Input folder containing screenshots (defaults to --src or current directory)",
    )
    parser.add_argument(
        "--src",
        type=Path,
        default=None,
        help="Input directory path containing screenshots",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=Path.home() / "Pictures" / "steam-cropped",
        help="Output directory path (defaults to ~/Pictures/steam-cropped)",
    )
    parser.add_argument(
        "--ratio",
        default="16:9",
        help="Target aspect ratio, e.g. 16:9 or 21:9 (default: 16:9)",
    )
    parser.add_argument(
        "--threshold",
        type=int,
        default=DEFAULT_BLACK_THRESHOLD,
        help=f"Black pixel luminance threshold between 0 and 255 (default: {DEFAULT_BLACK_THRESHOLD})",
    )
    parser.add_argument(
        "--quality",
        type=int,
        default=95,
        help="JPEG output quality between 1 and 100 (default: 95)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Simulate the crop process without writing files",
    )
    args = parser.parse_args()

    # Determine input directory
    source_dir = args.source or args.src
    if source_dir is None:
        # Fallback to local Steam screenshots default if present, or current directory
        steam_default = Path(r"C:\Program Files (x86)\Steam\userdata\174288491\760\remote\1875580\screenshots")
        source_dir = steam_default if steam_default.is_dir() else Path(".")

    if not source_dir.is_dir():
        print(f"Error: Source directory '{source_dir}' does not exist.", file=sys.stderr)
        sys.exit(1)

    target_ratio = parse_aspect_ratio(args.ratio)

    if not args.dry_run:
        args.out.mkdir(parents=True, exist_ok=True)

    files = sorted(
        f for f in source_dir.iterdir()
        if f.is_file() and f.suffix.lower() in SUPPORTED_EXTENSIONS
    )

    if not files:
        print(f"No image files found in '{source_dir}'.")
        return

    processed_count = 0
    skipped_count = 0

    for file_path in files:
        target_file = args.out / file_path.name
        if target_file.exists() and not args.dry_run:
            skipped_count += 1
            continue

        result = process_screenshot(
            image_path=file_path,
            output_dir=args.out,
            target_ratio=target_ratio,
            threshold=args.threshold,
            quality=args.quality,
            dry_run=args.dry_run,
        )
        print(f"[{file_path.name}] {result}")
        processed_count += 1

    print(f"\nDone! Processed: {processed_count}, Skipped (already existed): {skipped_count}")
    if not args.dry_run:
        print(f"Output saved to: {args.out}")


if __name__ == "__main__":
    main()

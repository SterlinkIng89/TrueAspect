# Steam Screenshot Cropper

Automate black bar removal from Steam screenshots taken on Ultrawide (21:9 / 32:9) monitors when playing games that only render in 16:9 (or games with letterboxed cutscenes).

## Overview

When playing games without native ultrawide support on a 21:9 or 32:9 display, screenshots captured by Steam often contain large black pillarbox bars on the left and right. Manually cropping dozens or hundreds of screenshots in photo editors is tedious.

**Steam Screenshot Cropper** automatically:
- Inspects your screenshot dimensions and desired target ratio (e.g. `16:9`, `21:9`).
- Verifies that the margins to be cropped actually consist of black pixels (so in-game dark elements or wider scenes are never accidentally clipped).
- Crops pillarboxed/letterboxed bars cleanly.
- Preserves original files without overwriting them.
- Skips already-processed screenshots on subsequent runs.

---

## Installation

1. Clone this repository:
   ```bash
   git clone https://github.com/SterlinkIng89/steam-screenshot-cropper.git
   cd steam-screenshot-cropper
   ```

2. Install dependencies:
   ```bash
   pip install -r requirements.txt
   ```

---

## Usage

### Basic Usage
Process screenshots from a folder:
```bash
python crop_ss.py "C:\Program Files (x86)\Steam\userdata\<your-id>\760\remote\<game-id>\screenshots"
```

Output cropped images are saved to `~/Pictures/steam-cropped` by default.

### Custom Output Directory
```bash
python crop_ss.py "C:\path\to\screenshots" --out "C:\path\to\output"
```

### Dry Run (Preview without writing files)
```bash
python crop_ss.py "C:\path\to\screenshots" --dry-run
```

### Custom Aspect Ratio
For games with cinematic letterboxing (bars on top and bottom) or custom ratios:
```bash
python crop_ss.py "C:\path\to\screenshots" --ratio 21:9
python crop_ss.py "C:\path\to\screenshots" --ratio 4:3
```

### CLI Arguments

| Argument | Default | Description |
|---|---|---|
| `source` / `--src` | Steam default / `.` | Folder containing screenshot images |
| `--out` | `~/Pictures/steam-cropped` | Destination directory for cropped images |
| `--ratio` | `16:9` | Target aspect ratio (`16:9`, `21:9`, `4:3`, etc.) |
| `--threshold` | `20` | Max luminance value (0-255) considered "black" |
| `--quality` | `95` | JPEG quality for output images |
| `--dry-run` | `False` | Run simulation without creating or copying files |

---

## License

MIT

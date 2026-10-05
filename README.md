# Steam Screenshot Cropper

Automate black bar removal from Steam screenshots taken on Ultrawide (21:9 / 32:9) monitors when playing games that render in 16:9 (or games with letterboxed cutscenes).

Available as both a **standalone Desktop GUI** (Go + Wails + React + Tailwind) and a **Python CLI script**.

## Features

- **Drag & Drop**: Drag a folder or multiple images directly onto the window.
- **Bulk Selection**: Select all, clear, click to toggle, or Shift+Click for range selection.
- **Thumbnail Grid**: Fast thumbnail generation with in-memory caching and lazy loading.
- **Smart Cropping**: Checks margin luminance (<= 20) before cropping to ensure gameplay elements aren't cut off.
- **Safe by Default**: Original files are never modified or overwritten; existing outputs are skipped.
- **Aspect Ratios**: 16:9 (standard), 21:9 (ultrawide), 4:3 (retro), 16:10.
- **Configurable Output**: Defaults to `~/Pictures/steam-cropped`.

---

## Desktop Application (Go + Wails)

### Prerequisites

- [Go](https://go.dev/) 1.23+
- [Node.js](https://nodejs.org/) 18+
- [Wails CLI](https://wails.io/): `go install github.com/wailsapp/wails/v2/cmd/wails@latest`

### Development

Run live development with hot reload:
```bash
wails dev
```

### Build Executable

Produce a production single binary (`build/bin/steam-screenshot-cropper.exe`):
```bash
wails build
```

---

## Python CLI Usage

If you prefer using the command-line script:

### Installation
```bash
pip install -r requirements.txt
```

### Basic Usage
```bash
python crop_ss.py "C:\Program Files (x86)\Steam\userdata\<your-id>\760\remote\<game-id>\screenshots"
```

### Options
```bash
python crop_ss.py "C:\path\to\screenshots" --out "C:\path\to\output" --ratio 16:9
```

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

# True Aspect

Automate black bar removal from screenshots taken on Ultrawide (21:9 / 32:9) monitors when playing games that render in 16:9 (or games with letterboxed cutscenes).

Built with **Rust + Tauri v2 + React + Tailwind**.

## Features

- **Drag & Drop**: Drag a folder or multiple images directly onto the window.
- **Bulk Selection**: Select all, clear, click to toggle, or Shift+Click for range selection.
- **Thumbnail Grid**: Fast thumbnail generation with in-memory caching and lazy loading.
- **Smart Cropping**: Checks margin luminance (<= 20) before cropping to ensure gameplay elements aren't cut off.
- **Safe by Default**: Original files are never modified or overwritten; existing outputs are skipped.
- **Aspect Ratios**: 16:9 (standard), 21:9 (ultrawide), 4:3 (retro), 16:10.
- **Configurable Output**: Defaults to `~/Pictures/steam-cropped`.

---

## Desktop Application (Rust + Tauri v2)

### Prerequisites

- [Rust](https://www.rust-lang.org/) (1.78+)
- [Node.js](https://nodejs.org/) 18+
- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (Desktop development with C++)

### Development

Run live development with hot reload:

```bash
npx @tauri-apps/cli dev
```

### Build Executable

Produce a production single binary:

```bash
npx @tauri-apps/cli build
```

---

## License

MIT

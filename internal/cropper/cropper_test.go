package cropper

import (
	"image"
	"image/color"
	"image/draw"
	"image/jpeg"
	"os"
	"path/filepath"
	"testing"
)

func TestParseRatio(t *testing.T) {
	tests := []struct {
		input    string
		expected float64
		hasError bool
	}{
		{"16:9", 16.0 / 9.0, false},
		{"21:9", 21.0 / 9.0, false},
		{"4:3", 4.0 / 3.0, false},
		{"16:10", 16.0 / 10.0, false},
		{"1.7777", 1.7777, false},
		{"invalid", 0, true},
		{"16:0", 0, true},
	}

	for _, tt := range tests {
		got, err := ParseRatio(tt.input)
		if (err != nil) != tt.hasError {
			t.Errorf("ParseRatio(%q) error = %v, wantErr %v", tt.input, err, tt.hasError)
			continue
		}
		if !tt.hasError && (got-tt.expected > 0.0001 || tt.expected-got > 0.0001) {
			t.Errorf("ParseRatio(%q) = %v, want %v", tt.input, got, tt.expected)
		}
	}
}

func TestTargetBox(t *testing.T) {
	// Ultrawide 3440x1440 with target 16:9 (ratio 16/9 ~ 1.7777)
	// Target width: 1440 * (16/9) = 2560.
	// Offset X = (3440 - 2560) / 2 = 440.
	box := TargetBox(3440, 1440, 16.0/9.0)
	expected := image.Rect(440, 0, 3000, 1440)
	if box != expected {
		t.Errorf("TargetBox(3440, 1440, 16:9) = %v, want %v", box, expected)
	}

	// Already 16:9 (1920x1080)
	box169 := TargetBox(1920, 1080, 16.0/9.0)
	expected169 := image.Rect(0, 0, 1920, 1080)
	if box169 != expected169 {
		t.Errorf("TargetBox(1920, 1080, 16:9) = %v, want %v", box169, expected169)
	}
}

func createTestImage(t *testing.T, w, h int, barWidth int, barColor color.Color, centerColor color.Color) image.Image {
	t.Helper()
	img := image.NewRGBA(image.Rect(0, 0, w, h))
	// Fill whole with centerColor
	draw.Draw(img, img.Bounds(), &image.Uniform{C: centerColor}, image.Point{}, draw.Src)
	// Left bar
	if barWidth > 0 {
		draw.Draw(img, image.Rect(0, 0, barWidth, h), &image.Uniform{C: barColor}, image.Point{}, draw.Src)
		draw.Draw(img, image.Rect(w-barWidth, 0, w, h), &image.Uniform{C: barColor}, image.Point{}, draw.Src)
	}
	return img
}

func saveJPEG(t *testing.T, path string, img image.Image) {
	t.Helper()
	f, err := os.Create(path)
	if err != nil {
		t.Fatalf("failed to create image file: %v", err)
	}
	defer f.Close()
	if err := jpeg.Encode(f, img, &jpeg.Options{Quality: 100}); err != nil {
		t.Fatalf("failed to encode jpeg: %v", err)
	}
}

func TestProcessImage(t *testing.T) {
	tmpDir := t.TempDir()
	srcDir := filepath.Join(tmpDir, "source")
	outDir := filepath.Join(tmpDir, "output")
	_ = os.MkdirAll(srcDir, 0755)
	_ = os.MkdirAll(outDir, 0755)

	// 1. Image with black pillarbox bars
	// 3440x1440, 440px black bars on left/right, bright content in middle
	blackBarImg := createTestImage(t, 3440, 1440, 440, color.Black, color.RGBA{R: 200, G: 150, B: 100, A: 255})
	blackBarPath := filepath.Join(srcDir, "screenshot_black.jpg")
	saveJPEG(t, blackBarPath, blackBarImg)

	res := Process(blackBarPath, outDir, 16.0/9.0, DefaultThreshold, 95)
	if res.Status != StatusCropped {
		t.Errorf("expected StatusCropped, got %s (%s)", res.Status, res.Message)
	}

	// Verify cropped image dimensions
	croppedFile, err := os.Open(filepath.Join(outDir, "screenshot_black.jpg"))
	if err != nil {
		t.Fatalf("cropped output file does not exist: %v", err)
	}
	defer croppedFile.Close()
	cfg, _, err := image.DecodeConfig(croppedFile)
	if err != nil {
		t.Fatalf("failed to read cropped image config: %v", err)
	}
	if cfg.Width != 2560 || cfg.Height != 1440 {
		t.Errorf("cropped dimensions = %dx%d, want 2560x1440", cfg.Width, cfg.Height)
	}

	// 2. Image with non-black bars (e.g. gameplay UI in margins) -> should copy without cropping
	coloredBarImg := createTestImage(t, 3440, 1440, 440, color.RGBA{R: 120, G: 80, B: 80, A: 255}, color.RGBA{R: 200, G: 150, B: 100, A: 255})
	coloredBarPath := filepath.Join(srcDir, "screenshot_colored.jpg")
	saveJPEG(t, coloredBarPath, coloredBarImg)

	resColored := Process(coloredBarPath, outDir, 16.0/9.0, DefaultThreshold, 95)
	if resColored.Status != StatusCopied {
		t.Errorf("expected StatusCopied, got %s (%s)", resColored.Status, resColored.Message)
	}

	// 3. Skip if already exists in output
	resSkip := Process(blackBarPath, outDir, 16.0/9.0, DefaultThreshold, 95)
	if resSkip.Status != StatusSkipped {
		t.Errorf("expected StatusSkipped for already existing output, got %s", resSkip.Status)
	}
}

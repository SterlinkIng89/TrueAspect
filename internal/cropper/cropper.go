package cropper

import (
	"fmt"
	"image"
	"image/draw"
	"image/jpeg"
	"image/png"
	"io"
	"math"
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

const DefaultThreshold = 20

type Status string

const (
	StatusCropped Status = "cropped"
	StatusCopied  Status = "copied"
	StatusSkipped Status = "skipped"
	StatusError   Status = "error"
)

type Result struct {
	Path    string `json:"path"`
	Status  Status `json:"status"`
	Message string `json:"message"`
}

func ParseRatio(s string) (float64, error) {
	s = strings.TrimSpace(s)
	if strings.Contains(s, ":") {
		parts := strings.Split(s, ":")
		if len(parts) != 2 {
			return 0, fmt.Errorf("invalid ratio format %q", s)
		}
		num, err1 := strconv.ParseFloat(strings.TrimSpace(parts[0]), 64)
		den, err2 := strconv.ParseFloat(strings.TrimSpace(parts[1]), 64)
		if err1 != nil || err2 != nil || den <= 0 || num <= 0 {
			return 0, fmt.Errorf("invalid ratio values in %q", s)
		}
		return num / den, nil
	}

	val, err := strconv.ParseFloat(s, 64)
	if err != nil || val <= 0 {
		return 0, fmt.Errorf("invalid ratio value %q", s)
	}
	return val, nil
}

func TargetBox(width, height int, targetRatio float64) image.Rectangle {
	if width <= 0 || height <= 0 || targetRatio <= 0 {
		return image.Rect(0, 0, width, height)
	}

	currentRatio := float64(width) / float64(height)
	if currentRatio > targetRatio {
		// Pillarbox (bars on left and right)
		targetWidth := int(math.Round(float64(height) * targetRatio))
		if targetWidth > width {
			targetWidth = width
		}
		offsetX := (width - targetWidth) / 2
		return image.Rect(offsetX, 0, offsetX+targetWidth, height)
	}

	targetHeight := int(math.Round(float64(width) / targetRatio))
	if targetHeight > height {
		targetHeight = height
	}
	offsetY := (height - targetHeight) / 2
	return image.Rect(0, offsetY, width, offsetY+targetHeight)
}

func IsBlackRegion(img image.Image, r image.Rectangle, threshold uint8) bool {
	inter := r.Intersect(img.Bounds())
	if inter.Empty() {
		return true
	}

	thresh16 := uint32(threshold)

	for y := inter.Min.Y; y < inter.Max.Y; y++ {
		for x := inter.Min.X; x < inter.Max.X; x++ {
			c := img.At(x, y)
			r32, g32, b32, _ := c.RGBA()
			r8 := r32 >> 8
			g8 := g32 >> 8
			b8 := b32 >> 8
			yLum := (299*r8 + 587*g8 + 114*b8) / 1000
			if yLum > thresh16 {
				return false
			}
		}
	}
	return true
}

func copyFile(src, dst string) error {
	in, err := os.Open(src)
	if err != nil {
		return err
	}
	defer in.Close()

	out, err := os.Create(dst)
	if err != nil {
		return err
	}
	defer out.Close()

	if _, err = io.Copy(out, in); err != nil {
		return err
	}
	return out.Sync()
}

func subImage(img image.Image, rect image.Rectangle) image.Image {
	type subImager interface {
		SubImage(r image.Rectangle) image.Image
	}
	if si, ok := img.(subImager); ok {
		return si.SubImage(rect)
	}

	// Fallback for types not implementing SubImage
	dst := image.NewRGBA(image.Rect(0, 0, rect.Dx(), rect.Dy()))
	draw.Draw(dst, dst.Bounds(), img, rect.Min, draw.Src)
	return dst
}

func Process(srcPath, outDir string, targetRatio float64, threshold uint8, quality int) Result {
	filename := filepath.Base(srcPath)
	destPath := filepath.Join(outDir, filename)

	if err := os.MkdirAll(outDir, 0755); err != nil {
		return Result{Path: srcPath, Status: StatusError, Message: fmt.Sprintf("failed to create output dir: %v", err)}
	}

	// Skip if already exists in output directory
	if _, err := os.Stat(destPath); err == nil {
		return Result{Path: srcPath, Status: StatusSkipped, Message: "already exists in output"}
	}

	inFile, err := os.Open(srcPath)
	if err != nil {
		return Result{Path: srcPath, Status: StatusError, Message: fmt.Sprintf("failed to open: %v", err)}
	}
	defer inFile.Close()

	img, format, err := image.Decode(inFile)
	if err != nil {
		return Result{Path: srcPath, Status: StatusError, Message: fmt.Sprintf("decode error: %v", err)}
	}

	bounds := img.Bounds()
	width := bounds.Dx()
	height := bounds.Dy()
	box := TargetBox(width, height, targetRatio)

	// If box equals full image dimensions, no crop is needed
	if box.Min.X == 0 && box.Min.Y == 0 && box.Dx() == width && box.Dy() == height {
		if err := copyFile(srcPath, destPath); err != nil {
			return Result{Path: srcPath, Status: StatusError, Message: fmt.Sprintf("copy error: %v", err)}
		}
		return Result{Path: srcPath, Status: StatusCopied, Message: "already target ratio; copied"}
	}

	// Calculate 4 candidate margin regions to verify they are black
	leftBar := image.Rect(0, 0, box.Min.X, height)
	rightBar := image.Rect(box.Max.X, 0, width, height)
	topBar := image.Rect(0, 0, width, box.Min.Y)
	bottomBar := image.Rect(0, box.Max.Y, width, height)

	barsAreBlack := IsBlackRegion(img, leftBar, threshold) &&
		IsBlackRegion(img, rightBar, threshold) &&
		IsBlackRegion(img, topBar, threshold) &&
		IsBlackRegion(img, bottomBar, threshold)

	if !barsAreBlack {
		if err := copyFile(srcPath, destPath); err != nil {
			return Result{Path: srcPath, Status: StatusError, Message: fmt.Sprintf("copy error: %v", err)}
		}
		return Result{Path: srcPath, Status: StatusCopied, Message: "non-black margins; copied without cropping"}
	}

	// Crop image
	cropped := subImage(img, box)

	outFile, err := os.Create(destPath)
	if err != nil {
		return Result{Path: srcPath, Status: StatusError, Message: fmt.Sprintf("failed to create output file: %v", err)}
	}
	defer outFile.Close()

	ext := strings.ToLower(filepath.Ext(filename))
	if ext == ".png" || format == "png" {
		if err := png.Encode(outFile, cropped); err != nil {
			return Result{Path: srcPath, Status: StatusError, Message: fmt.Sprintf("png encode error: %v", err)}
		}
	} else {
		q := quality
		if q <= 0 || q > 100 {
			q = 95
		}
		if err := jpeg.Encode(outFile, cropped, &jpeg.Options{Quality: q}); err != nil {
			return Result{Path: srcPath, Status: StatusError, Message: fmt.Sprintf("jpeg encode error: %v", err)}
		}
	}

	return Result{
		Path:    srcPath,
		Status:  StatusCropped,
		Message: fmt.Sprintf("cropped %dx%d -> %dx%d", width, height, box.Dx(), box.Dy()),
	}
}


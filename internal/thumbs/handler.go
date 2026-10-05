package thumbs

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"image"
	"image/jpeg"
	_ "image/png"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"sync"

	"golang.org/x/image/draw"
)

type Handler struct {
	cache      sync.Map
	allowed    sync.Map
	thumbWidth int
}

func NewHandler(thumbWidth int) *Handler {
	if thumbWidth <= 0 {
		thumbWidth = 400
	}
	return &Handler{
		thumbWidth: thumbWidth,
	}
}

// RegisterAllowed marks paths as permitted for thumbnail generation.
func (h *Handler) RegisterAllowed(paths []string) {
	for _, p := range paths {
		h.allowed.Store(filepath.Clean(p), true)
	}
}

// ClearAllowed clears the allowed paths map.
func (h *Handler) ClearAllowed() {
	h.allowed.Range(func(key, _ any) bool {
		h.allowed.Delete(key)
		return true
	})
	h.cache.Range(func(key, _ any) bool {
		h.cache.Delete(key)
		return true
	})
}

func (h *Handler) isAllowed(p string) bool {
	clean := filepath.Clean(p)
	if _, ok := h.allowed.Load(clean); ok {
		return true
	}
	ext := strings.ToLower(filepath.Ext(clean))
	if ext == ".jpg" || ext == ".jpeg" || ext == ".png" {
		if _, err := os.Stat(clean); err == nil {
			return true
		}
	}
	return false
}

func (h *Handler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	filePath := r.URL.Query().Get("path")
	if filePath == "" {
		http.Error(w, "missing path parameter", http.StatusBadRequest)
		return
	}

	cleanPath := filepath.Clean(filePath)
	if !h.isAllowed(cleanPath) {
		http.Error(w, "forbidden or invalid file", http.StatusForbidden)
		return
	}

	fileInfo, err := os.Stat(cleanPath)
	if err != nil {
		http.Error(w, "file not found", http.StatusNotFound)
		return
	}

	hashInput := cleanPath + fileInfo.ModTime().String()
	hasher := sha256.New()
	hasher.Write([]byte(hashInput))
	cacheKey := hex.EncodeToString(hasher.Sum(nil))

	if cached, ok := h.cache.Load(cacheKey); ok {
		if data, ok := cached.([]byte); ok {
			w.Header().Set("Content-Type", "image/jpeg")
			w.Header().Set("Cache-Control", "public, max-age=86400")
			_, _ = w.Write(data)
			return
		}
	}

	f, err := os.Open(cleanPath)
	if err != nil {
		http.Error(w, "cannot open file", http.StatusInternalServerError)
		return
	}
	defer f.Close()

	srcImg, _, err := image.Decode(f)
	if err != nil {
		http.Error(w, "cannot decode image", http.StatusInternalServerError)
		return
	}

	srcBounds := srcImg.Bounds()
	origW := srcBounds.Dx()
	origH := srcBounds.Dy()

	targetW := h.thumbWidth
	if targetW > origW {
		targetW = origW
	}
	targetH := int(float64(origH) * (float64(targetW) / float64(origW)))
	if targetH <= 0 {
		targetH = 1
	}

	dstImg := image.NewRGBA(image.Rect(0, 0, targetW, targetH))
	draw.ApproxBiLinear.Scale(dstImg, dstImg.Bounds(), srcImg, srcBounds, draw.Src, nil)

	var buf bytes.Buffer
	if err := jpeg.Encode(&buf, dstImg, &jpeg.Options{Quality: 80}); err != nil {
		http.Error(w, "cannot encode thumbnail", http.StatusInternalServerError)
		return
	}

	data := buf.Bytes()
	h.cache.Store(cacheKey, data)

	w.Header().Set("Content-Type", "image/jpeg")
	w.Header().Set("Cache-Control", "public, max-age=86400")
	_, _ = w.Write(data)
}

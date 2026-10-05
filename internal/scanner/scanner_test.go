package scanner

import (
	"os"
	"path/filepath"
	"testing"
)

func TestScan(t *testing.T) {
	tmp := t.TempDir()
	img1 := filepath.Join(tmp, "a.jpg")
	img2 := filepath.Join(tmp, "b.PNG")
	txt := filepath.Join(tmp, "notes.txt")
	subDir := filepath.Join(tmp, "subdir")

	_ = os.WriteFile(img1, []byte("data"), 0644)
	_ = os.WriteFile(img2, []byte("data"), 0644)
	_ = os.WriteFile(txt, []byte("ignore"), 0644)
	_ = os.Mkdir(subDir, 0755)

	// Scan folder
	results, err := Scan([]string{tmp})
	if err != nil {
		t.Fatalf("Scan error: %v", err)
	}

	if len(results) != 2 {
		t.Fatalf("expected 2 images, got %d", len(results))
	}
	if results[0].Name != "a.jpg" || results[1].Name != "b.PNG" {
		t.Errorf("unexpected results: %+v", results)
	}

	// Scan single files with deduplication
	results2, err := Scan([]string{img1, img1, img2})
	if err != nil {
		t.Fatalf("Scan error: %v", err)
	}
	if len(results2) != 2 {
		t.Fatalf("expected 2 deduped images, got %d", len(results2))
	}
}

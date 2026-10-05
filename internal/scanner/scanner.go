package scanner

import (
	"os"
	"path/filepath"
	"sort"
	"strings"
)

type Screenshot struct {
	Path string `json:"path"`
	Name string `json:"name"`
}

var supportedExtensions = map[string]bool{
	".jpg":  true,
	".jpeg": true,
	".png":  true,
}

func isSupported(ext string) bool {
	return supportedExtensions[strings.ToLower(ext)]
}

func Scan(paths []string) ([]Screenshot, error) {
	seen := make(map[string]bool)
	var screenshots []Screenshot

	addFile := func(filePath string) {
		clean := filepath.Clean(filePath)
		if seen[clean] {
			return
		}
		ext := filepath.Ext(clean)
		if isSupported(ext) {
			seen[clean] = true
			screenshots = append(screenshots, Screenshot{
				Path: clean,
				Name: filepath.Base(clean),
			})
		}
	}

	for _, p := range paths {
		info, err := os.Stat(p)
		if err != nil {
			continue
		}

		if info.IsDir() {
			entries, err := os.ReadDir(p)
			if err != nil {
				continue
			}
			for _, entry := range entries {
				if !entry.IsDir() {
					addFile(filepath.Join(p, entry.Name()))
				}
			}
		} else {
			addFile(p)
		}
	}

	sort.Slice(screenshots, func(i, j int) bool {
		return strings.ToLower(screenshots[i].Name) < strings.ToLower(screenshots[j].Name)
	})

	return screenshots, nil
}

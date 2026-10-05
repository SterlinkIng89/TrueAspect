package main

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	stdRuntime "runtime"
	"sync"
	"sync/atomic"

	"steam-screenshot-cropper/internal/cropper"
	"steam-screenshot-cropper/internal/scanner"
	"steam-screenshot-cropper/internal/thumbs"

	"github.com/wailsapp/wails/v2/pkg/runtime"
)

type App struct {
	ctx          context.Context
	thumbHandler *thumbs.Handler
}

func NewApp() *App {
	return &App{
		thumbHandler: thumbs.NewHandler(420),
	}
}

func (a *App) startup(ctx context.Context) {
	a.ctx = ctx
	runtime.OnFileDrop(ctx, func(x, y int, paths []string) {
		runtime.EventsEmit(ctx, "files:dropped", paths)
	})
}

func (a *App) GetDefaultOutputDir() string {
	home, err := os.UserHomeDir()
	if err != nil {
		return filepath.Join(".", "steam-cropped")
	}
	return filepath.Join(home, "Pictures", "steam-cropped")
}

func (a *App) PickFolder() (string, error) {
	selected, err := runtime.OpenDirectoryDialog(a.ctx, runtime.OpenDialogOptions{
		Title: "Select screenshots folder",
	})
	if err != nil {
		return "", err
	}
	return selected, nil
}

func (a *App) PickOutputFolder() (string, error) {
	selected, err := runtime.OpenDirectoryDialog(a.ctx, runtime.OpenDialogOptions{
		Title: "Select output folder",
	})
	if err != nil {
		return "", err
	}
	return selected, nil
}

func (a *App) LoadScreenshots(paths []string) ([]scanner.Screenshot, error) {
	if len(paths) == 0 {
		return []scanner.Screenshot{}, nil
	}

	results, err := scanner.Scan(paths)
	if err != nil {
		return nil, err
	}

	allowed := make([]string, len(results))
	for i, r := range results {
		allowed[i] = r.Path
	}
	a.thumbHandler.RegisterAllowed(allowed)

	return results, nil
}

type CropProgressPayload struct {
	Current int    `json:"current"`
	Total   int    `json:"total"`
	Name    string `json:"name"`
	Status  string `json:"status"`
}

func (a *App) CropScreenshots(paths []string, ratioStr string, outDir string) ([]cropper.Result, error) {
	if len(paths) == 0 {
		return []cropper.Result{}, nil
	}

	if outDir == "" {
		outDir = a.GetDefaultOutputDir()
	}

	ratio, err := cropper.ParseRatio(ratioStr)
	if err != nil {
		return nil, fmt.Errorf("invalid ratio %q: %w", ratioStr, err)
	}

	total := len(paths)
	results := make([]cropper.Result, total)

	type job struct {
		index int
		path  string
	}

	concurrency := stdRuntime.NumCPU()
	if concurrency > 4 {
		concurrency = 4
	}
	if concurrency > total {
		concurrency = total
	}
	if concurrency <= 0 {
		concurrency = 1
	}

	jobs := make(chan job, total)
	for i, p := range paths {
		jobs <- job{index: i, path: p}
	}
	close(jobs)

	var wg sync.WaitGroup
	var completed atomic.Int32

	for w := 0; w < concurrency; w++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for j := range jobs {
				res := cropper.Process(j.path, outDir, ratio, cropper.DefaultThreshold, 95)
				results[j.index] = res

				done := int(completed.Add(1))
				runtime.EventsEmit(a.ctx, "crop:progress", CropProgressPayload{
					Current: done,
					Total:   total,
					Name:    filepath.Base(j.path),
					Status:  "processing",
				})
			}
		}()
	}
	wg.Wait()

	runtime.EventsEmit(a.ctx, "crop:done", map[string]any{
		"total": total,
	})

	return results, nil
}

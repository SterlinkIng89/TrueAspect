package main

import (
	"embed"

	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/assetserver"
)

//go:embed all:frontend/dist
var assets embed.FS

func main() {
	app := NewApp()

	err := wails.Run(&options.App{
		Title:  "Steam Screenshot Cropper",
		Width:  1160,
		Height: 780,
		MinWidth: 800,
		MinHeight: 560,
		AssetServer: &assetserver.Options{
			Assets:  assets,
			Handler: app.thumbHandler,
		},
		BackgroundColour: &options.RGBA{R: 14, G: 15, B: 18, A: 1},
		OnStartup:        app.startup,
		DragAndDrop: &options.DragAndDrop{
			EnableFileDrop: true,
		},
		Bind: []interface{}{
			app,
		},
	})

	if err != nil {
		println("Error:", err.Error())
	}
}

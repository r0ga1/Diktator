//go:build windows

package main

import (
	"encoding/json"
	webview "github.com/jchv/go-webview2"
	"log"
	"net/url"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync/atomic"
	"syscall"
	"time"
	"unsafe"
)

func main() {
	runtime.LockOSThread()
	log.SetOutput(os.Stderr)
	log.Print("Silbenlot UI 0.12.0: main entered")
	exe, e := os.Executable()
	if e != nil {
		log.Print(e)
		os.Exit(45)
	}
	ui := filepath.Dir(exe)
	fixed := filepath.Join(filepath.Dir(ui), "webview2")
	html := filepath.Join(ui, "index.html")
	for _, p := range []string{html, filepath.Join(ui, "WebView2Loader.dll"), filepath.Join(fixed, "msedgewebview2.exe")} {
		if _, e = os.Stat(p); e != nil {
			log.Print(e)
			os.Exit(44)
		}
	}
	os.Setenv("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", fixed)
	os.Setenv("WEBVIEW2_DEFAULT_BACKGROUND_COLOR", "FF1E1E20")
	os.Setenv("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "--disable-background-networking --disable-component-update --disable-sync --no-first-run")
	appID, _ := syscall.UTF16PtrFromString("Silbenlot.Desktop")
	syscall.NewLazyDLL("shell32.dll").NewProc("SetCurrentProcessExplicitAppUserModelID").Call(uintptr(unsafe.Pointer(appID)))
	c := connect(os.Stdin, os.Stdout)
	initTimer := time.AfterFunc(30*time.Second, func() { log.Print("ERROR: WebView2 controller initialization timed out"); os.Exit(46) })
	w := webview.NewWithOptions(webview.WebViewOptions{DataPath: filepath.Join(os.Getenv("LOCALAPPDATA"), "Silbenlot", "WebView2-0.8"), AutoFocus: true, WindowOptions: webview.WindowOptions{IconId: 1, Title: "Silbenlot wird gestartet …", Width: 1140, Height: 810, Center: true, Hidden: false}, NavigationCompleted: func(ok bool, status int32, e error) {
		log.Printf("Navigation completed: success=%v webError=%d error=%v", ok, status, e)
		// OperationCanceled (14) is normal when closing during navigation.
		if e != nil || (!ok && status != 14) {
			os.Exit(48)
		}
	}})
	initTimer.Stop()
	if w == nil {
		log.Print("ERROR: WebView2 controller creation failed")
		os.Exit(44)
	}
	defer w.Destroy()
	w.SetSize(880, 630, webview.HintMin)
	dark := int32(1)
	syscall.NewLazyDLL("dwmapi.dll").NewProc("DwmSetWindowAttribute").Call(uintptr(w.Window()), 20, uintptr(unsafe.Pointer(&dark)), 4)
	var ready atomic.Bool
	readyTimer := time.AfterFunc(20*time.Second, func() {
		if !ready.Load() {
			log.Print("ERROR: dashboard did not confirm rendered content and connected core within 20s")
			os.Exit(47)
		}
	})
	defer readyTimer.Stop()
	if e = w.Bind("nativeReady", func() bool {
		if ready.CompareAndSwap(false, true) {
			readyTimer.Stop()
			log.Print("READY: dashboard rendered, core connected")
			w.Dispatch(func() { w.SetTitle("Silbenlot") })
		}
		return true
	}); e != nil {
		log.Print(e)
		os.Exit(49)
	}
	if e = w.Bind("nativeLog", func(message string) {
		if len(message) > 1000 {
			message = message[:1000]
		}
		log.Print("Page: ", message)
	}); e != nil {
		log.Print(e)
		os.Exit(49)
	}
	if e = w.Bind("nativeCall", func(method, params string) (json.RawMessage, error) { return c.call(method, json.RawMessage(params)) }); e != nil {
		log.Print(e)
		os.Exit(49)
	}
	id, _ := json.Marshal(uintptr(w.Window()))
	if _, e = c.call("attachWindow", id); e != nil {
		log.Print(e)
		os.Exit(49)
	}
	path := strings.ReplaceAll(html, "\\", "/")
	if !strings.HasPrefix(path, "/") {
		path = "/" + path
	}
	uri := (&url.URL{Scheme: "file", Path: path}).String()
	log.Print("Loading bundle-verified local page: ", uri)
	w.Navigate(uri)
	w.Run()
}

package webviewloader

import (
	"fmt"
	"golang.org/x/sys/windows"
	"os"
	"path/filepath"
	"unsafe"
)

func native() (*windows.LazyDLL, error) {
	exe, e := os.Executable()
	if e != nil {
		return nil, e
	}
	d := windows.NewLazyDLL(filepath.Join(filepath.Dir(exe), "WebView2Loader.dll"))
	if e = d.Load(); e != nil {
		return nil, e
	}
	return d, nil
}
func call(name string, args ...uintptr) (uintptr, error) {
	d, e := native()
	if e != nil {
		return 0, e
	}
	p := d.NewProc(name)
	if e = p.Find(); e != nil {
		return 0, e
	}
	hr, _, _ := p.Call(args...)
	if int32(hr) < 0 {
		return hr, fmt.Errorf("%s HRESULT 0x%08X", name, uint32(hr))
	}
	return hr, nil
}
func CompareBrowserVersions(a, b string) (int, error) {
	aa, e := windows.UTF16PtrFromString(a)
	if e != nil {
		return 0, e
	}
	bb, e := windows.UTF16PtrFromString(b)
	if e != nil {
		return 0, e
	}
	var n int32
	_, e = call("CompareBrowserVersions", uintptr(unsafe.Pointer(aa)), uintptr(unsafe.Pointer(bb)), uintptr(unsafe.Pointer(&n)))
	return int(n), e
}
func GetInstalledVersion() (string, error) {
	var p *uint16
	_, e := call("GetAvailableCoreWebView2BrowserVersionString", 0, uintptr(unsafe.Pointer(&p)))
	if p != nil {
		defer windows.CoTaskMemFree(unsafe.Pointer(p))
	}
	return windows.UTF16PtrToString(p), e
}
func CreateCoreWebView2EnvironmentWithOptions(folder, data *uint16, options, callback uintptr) (uintptr, error) {
	return call("CreateCoreWebView2EnvironmentWithOptions", uintptr(unsafe.Pointer(folder)), uintptr(unsafe.Pointer(data)), options, callback)
}

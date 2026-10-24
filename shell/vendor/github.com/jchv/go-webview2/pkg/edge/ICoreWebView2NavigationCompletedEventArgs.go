package edge

import (
	"fmt"
	"unsafe"
)

type _ICoreWebView2NavigationCompletedEventArgsVtbl struct {
	_IUnknownVtbl
	GetIsSuccess      ComProc
	GetWebErrorStatus ComProc
	GetNavigationId   ComProc
}

type ICoreWebView2NavigationCompletedEventArgs struct {
	vtbl *_ICoreWebView2NavigationCompletedEventArgsVtbl
}

func (i *ICoreWebView2NavigationCompletedEventArgs) AddRef() uintptr {
	r, _, _ := i.vtbl.AddRef.Call()
	return r
}

func (i *ICoreWebView2NavigationCompletedEventArgs) Result() (bool, int32, error) {
	var success, status int32
	hr, _, _ := i.vtbl.GetIsSuccess.Call(uintptr(unsafe.Pointer(i)), uintptr(unsafe.Pointer(&success)))
	if int32(hr) < 0 {
		return false, 0, fmt.Errorf("get_IsSuccess HRESULT 0x%08X", uint32(hr))
	}
	hr, _, _ = i.vtbl.GetWebErrorStatus.Call(uintptr(unsafe.Pointer(i)), uintptr(unsafe.Pointer(&status)))
	if int32(hr) < 0 {
		return false, 0, fmt.Errorf("get_WebErrorStatus HRESULT 0x%08X", uint32(hr))
	}
	return success != 0, status, nil
}

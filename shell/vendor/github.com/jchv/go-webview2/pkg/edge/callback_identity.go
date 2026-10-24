package edge

import (
	"golang.org/x/sys/windows"
	"unsafe"
)

func callbackInterface(this, refiid, object uintptr, id string) uintptr {
	if object == 0 {
		return 0x80004003
	}
	*(*uintptr)(unsafe.Pointer(object)) = 0
	iid, e := windows.GUIDFromString(id)
	if e != nil {
		return 0x80004002
	}
	unknown := windows.GUID{Data4: [8]byte{0xc0, 0, 0, 0, 0, 0, 0, 0x46}}
	actual := *(*windows.GUID)(unsafe.Pointer(refiid))
	if actual != iid && actual != unknown {
		return 0x80004002
	}
	*(*uintptr)(unsafe.Pointer(object)) = this
	return 0
}

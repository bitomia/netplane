//go:generate make -C ../cpp/ bindgen

package gonetplane

/*
#cgo CFLAGS: -I.
#cgo LDFLAGS: -lnetplane_client
#cgo darwin LDFLAGS: -framework Security -framework SystemConfiguration
#include "netplane.h"
#include <stdint.h>
#include <stdlib.h>
*/
import "C"
import (
	"fmt"
	"unsafe"
)

// LogFormat selects the log output style.
type LogFormat uint32

const (
	LogFormatPretty LogFormat = 0
	LogFormatJSON   LogFormat = 1
	LogFormatLogfmt LogFormat = 2
)

// InitLogger initializes the netplane logger with the given output format.
func InitLogger(format LogFormat) {
	C.netplane_init_logger(C.uint32_t(format))
}

// ClientAuth authenticates the client with the server
func ClientAuth(authKeyPath, publicKeyPath, privateKeyPath, host, linkCode string, authPort uint16) error {
	cAuthKeyPath := C.CString(authKeyPath)
	defer C.free(unsafe.Pointer(cAuthKeyPath))

	cPublicKeyPath := C.CString(publicKeyPath)
	defer C.free(unsafe.Pointer(cPublicKeyPath))

	cPrivateKeyPath := C.CString(privateKeyPath)
	defer C.free(unsafe.Pointer(cPrivateKeyPath))

	cHost := C.CString(host)
	defer C.free(unsafe.Pointer(cHost))

	cLinkCode := C.CString(linkCode)
	defer C.free(unsafe.Pointer(cLinkCode))

	result := C.netplane_client_auth(
		cAuthKeyPath,
		cPublicKeyPath,
		cPrivateKeyPath,
		cHost,
		cLinkCode,
		C.uint16_t(authPort),
	)

	if result != 0 {
		return fmt.Errorf("client auth failed with code: %d", result)
	}

	return nil
}

// TryGenerateCryptoKeys generates crypto keys if they don't exist
func TryGenerateCryptoKeys(publicFilepath, privateFilepath string) error {
	cPublicFilepath := C.CString(publicFilepath)
	defer C.free(unsafe.Pointer(cPublicFilepath))

	cPrivateFilepath := C.CString(privateFilepath)
	defer C.free(unsafe.Pointer(cPrivateFilepath))

	result := C.netplane_try_generate_crypto_keys(cPublicFilepath, cPrivateFilepath)

	if result != 0 {
		return fmt.Errorf("failed to generate crypto keys with code: %d", result)
	}

	return nil
}

// Stop stops the running client
func Stop() {
	C.netplane_client_stop()
}

// CancelToken is an opaque handle to a cancellation token
type CancelToken struct {
	ptr unsafe.Pointer
}

// Run runs the netplane client with the given parameters and returns a CancelToken
func Run(tunDev, host string, port uint16, transportType, authKeyPath, publicKeyPath, privateKeyPath string) (*CancelToken, error) {
	cTunDev := C.CString(tunDev)
	defer C.free(unsafe.Pointer(cTunDev))

	cHost := C.CString(host)
	defer C.free(unsafe.Pointer(cHost))

	var cTransportType *C.char
	if transportType != "" {
		cTransportType = C.CString(transportType)
		defer C.free(unsafe.Pointer(cTransportType))
	}

	cAuthKeyPath := C.CString(authKeyPath)
	defer C.free(unsafe.Pointer(cAuthKeyPath))

	cPublicKeyPath := C.CString(publicKeyPath)
	defer C.free(unsafe.Pointer(cPublicKeyPath))

	cPrivateKeyPath := C.CString(privateKeyPath)
	defer C.free(unsafe.Pointer(cPrivateKeyPath))

	var tokenPtr unsafe.Pointer
	result := C.netplane_run(cTunDev, cHost, C.uint16_t(port), cTransportType, false, false, cAuthKeyPath, cPublicKeyPath, cPrivateKeyPath, &tokenPtr)

	if result != 0 {
		return nil, fmt.Errorf("run failed with code: %d", result)
	}

	return &CancelToken{ptr: tokenPtr}, nil
}

// Cancel cancels the operation associated with this token
func (t *CancelToken) Cancel() {
	if t != nil && t.ptr != nil {
		C.netplane_cancel(t.ptr)
	}
}

// Free releases the resources associated with this token
func (t *CancelToken) Free() {
	if t != nil && t.ptr != nil {
		C.netplane_free_cancel_token(t.ptr)
		t.ptr = nil
	}
}

package gonetplane

/*
#cgo CFLAGS: -I.
#cgo linux,arm64   LDFLAGS: -L../../target/aarch64-unknown-linux-gnu/release -lnetplane_client
#cgo linux,amd64   LDFLAGS: -L../../target/x86_64-unknown-linux-gnu/release -lnetplane_client
#cgo windows,amd64 LDFLAGS: -L../../target/x86_64-pc-windows-msvc/release -lnetplane_client
#cgo darwin LDFLAGS: -L../../target/universal-apple-darwin/release -lnetplane_client -framework Security -framework SystemConfiguration
#include "netplane.h"
#include <stdlib.h>
*/
import "C"
import (
	"errors"
	"fmt"
	"unsafe"
)

// InitLogger initializes the netplane logger
func InitLogger() {
	C.netplane_init_logger()
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

// Transport represents a netplane transport
type Transport struct {
	ptr unsafe.Pointer
}

// CreateTransport creates a new transport
func CreateTransport(serverAddr string, serverPort uint16, transportType string) (*Transport, error) {
	cServerAddr := C.CString(serverAddr)
	defer C.free(unsafe.Pointer(cServerAddr))

	var cTransportType *C.char
	if transportType != "" {
		cTransportType = C.CString(transportType)
		defer C.free(unsafe.Pointer(cTransportType))
	}

	ptr := C.netplane_create_transport(cServerAddr, C.uint16_t(serverPort), cTransportType)
	if ptr == nil {
		return nil, errors.New("failed to create transport")
	}

	return &Transport{ptr: ptr}, nil
}

// Free frees the transport
func (t *Transport) Free() {
	if t.ptr != nil {
		C.netplane_free_transport(t.ptr)
		t.ptr = nil
	}
}

// HandshakeResult contains the result of a handshake
type HandshakeResult struct {
	Netmask     string
	Destination string
	IPAddr      string
	cResult     *C.struct_NetplaneHandshakeResult
}

// ClientHandshake performs a handshake with the server
func ClientHandshake(authKeyPath string, transport *Transport) (*HandshakeResult, error) {
	if transport == nil || transport.ptr == nil {
		return nil, errors.New("transport is nil")
	}

	cAuthKeyPath := C.CString(authKeyPath)
	defer C.free(unsafe.Pointer(cAuthKeyPath))

	var cResult C.struct_NetplaneHandshakeResult
	result := C.netplane_client_handshake(cAuthKeyPath, transport.ptr, &cResult)

	if result != 0 {
		return nil, fmt.Errorf("handshake failed with code: %d", result)
	}

	handshake := &HandshakeResult{
		Netmask:     C.GoString(cResult.netmask),
		Destination: C.GoString(cResult.destination),
		IPAddr:      C.GoString(cResult.ip_addr),
		cResult:     &cResult,
	}

	return handshake, nil
}

// Free frees the handshake result
func (h *HandshakeResult) Free() {
	if h.cResult != nil {
		C.netplane_client_free_handshake(h.cResult)
		h.cResult = nil
	}
}

// ClientRun runs the client with the given TUN file descriptor
func ClientRun(tunFd int, transport *Transport, handshake *HandshakeResult) error {
	if transport == nil || transport.ptr == nil {
		return errors.New("transport is nil")
	}

	if handshake == nil || handshake.cResult == nil {
		return errors.New("handshake is nil")
	}

	result := C.netplane_client_run(C.int(tunFd), transport.ptr, handshake.cResult, false, false)

	if result != 0 {
		return fmt.Errorf("client run failed with code: %d", result)
	}

	return nil
}

// ClientStop stops the running client
func ClientStop() {
	C.netplane_client_stop()
}

// Run runs the netplane client with the given parameters
func Run(tunDev, host string, port uint16, transportType string) error {
	cTunDev := C.CString(tunDev)
	defer C.free(unsafe.Pointer(cTunDev))

	cHost := C.CString(host)
	defer C.free(unsafe.Pointer(cHost))

	var cTransportType *C.char
	if transportType != "" {
		cTransportType = C.CString(transportType)
		defer C.free(unsafe.Pointer(cTransportType))
	}

	result := C.netplane_run(cTunDev, cHost, C.uint16_t(port), cTransportType, false, false)

	if result != 0 {
		return fmt.Errorf("run failed with code: %d", result)
	}

	return nil
}

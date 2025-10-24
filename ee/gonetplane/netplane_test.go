package gonetplane_test

import (
	"os"
	"testing"

	"github.com/stretchr/testify/assert"

	netplane "github.com/bitomia/netplane/ee/gonetplane"
)

func TestNetplane(t *testing.T) {
	netplane.InitLogger()

	os.Remove("public.key")
	os.Remove("private.key")
	err := netplane.TryGenerateCryptoKeys("public.key", "private.key")
	assert.NoError(t, err)
	assert.FileExists(t, "public.key", "private.key")

	transport, err := netplane.CreateTransport("example.com", 5000, "udp")
	assert.NoError(t, err)
	defer transport.Free()
}

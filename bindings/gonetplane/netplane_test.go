package gonetplane_test

import (
	"os"
	"testing"

	"github.com/stretchr/testify/assert"

	netplane "github.com/bitomia/netplane/bindings/gonetplane"
)

func TestNetplane(t *testing.T) {
	netplane.ClientInitLogger(netplane.LogFormatPretty)

	os.Remove("public.key")
	os.Remove("private.key")
	err := netplane.TryGenerateCryptoKeys("public.key", "private.key")
	assert.NoError(t, err)
	assert.FileExists(t, "public.key", "private.key")

	err = netplane.ClientRun("tun0", "localhost", 12512, "ws", true, "./auth.key", "./public.key", "./private.key")
	assert.Error(t, err)
}

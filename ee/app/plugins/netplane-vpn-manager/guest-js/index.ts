import { invoke } from '@tauri-apps/api/core'

export async function ping(value: string): Promise<string | null> {
  return await invoke<{value?: string}>('plugin:netplane-vpn-manager|ping', {
    payload: {
      value,
    },
  }).then((r) => (r.value ? r.value : null));
}

export async function requestVpnPermission(): Promise<boolean> {
  return await invoke<{granted: boolean}>('plugin:netplane-vpn-manager|request_vpn_permission')
    .then((r) => r.granted);
}

export async function startVpn(address: string, routeAddress: string, prefixLength: number): Promise<number> {
  return await invoke<{fd: number}>('plugin:netplane-vpn-manager|start_vpn', {
    payload: {
      address,
      routeAddress,
      prefixLength,
    },
  }).then((r) => r.fd);
}

export async function stopVpn(): Promise<void> {
  await invoke('plugin:netplane-vpn-manager|stop_vpn');
}

export async function getTunnelFd(): Promise<number> {
  return await invoke<{fd: number}>('plugin:netplane-vpn-manager|get_tunnel_fd')
    .then((r) => r.fd);
}

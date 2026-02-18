package com.bitomia.netplane.vpn

import android.content.Context
import android.content.Intent
import android.net.VpnService
import android.util.Log

class NetplaneVpnManager(private val context: Context) {

    companion object {
        private const val TAG = "NetplaneVpnManager"
    }

    fun isVpnPermissionGranted(): Boolean {
        val intent = VpnService.prepare(context)
        return intent == null
    }

    fun startVpnService(): Intent {
        Log.d(TAG, "Starting VPN service")
        val intent = Intent(context, NetplaneVpnService::class.java)
        context.startService(intent)
        return intent
    }

    fun stopVpnService() {
        Log.d(TAG, "Stopping VPN service")
        val intent = Intent(context, NetplaneVpnService::class.java)
        context.stopService(intent)
        NetplaneVpnService.stopVpn()
    }

    fun createTunnelInterface(address: String, routeAddress: String, prefixLength: Int): Int {
        Log.d(TAG, "createTunnelInterface: waiting for service instance")
        val service = NetplaneVpnService.waitForInstance(3000)
        if (service == null) {
            Log.e(TAG, "VPN service instance is null after waiting")
            return -1
        }
        Log.d(TAG, "Got service instance, creating VPN interface")
        return service.createVpnInterface(address, routeAddress, prefixLength)
    }
}

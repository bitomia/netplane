package com.bitomia.netplane.vpn

import android.content.Context
import android.content.Intent
import android.net.VpnService

class NetplaneVpnManager(private val context: Context) {

    fun isVpnPermissionGranted(): Boolean {
        val intent = VpnService.prepare(context)
        return intent == null
    }

    fun startVpnService(): Intent {
        val intent = Intent(context, NetplaneVpnService::class.java)
        context.startService(intent)
        return intent
    }

    fun stopVpnService() {
        val intent = Intent(context, NetplaneVpnService::class.java)
        context.stopService(intent)
        NetplaneVpnService.stopVpn()
    }

    fun createTunnelInterface(address: String, routeAddress: String, prefixLength: Int): Int {
        val service = NetplaneVpnService.getInstance()
        return service?.createVpnInterface(address, routeAddress, prefixLength) ?: -1
    }
}

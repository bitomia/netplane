package com.bitomia.netplane.vpn

import android.content.Intent
import android.net.VpnService
import android.os.ParcelFileDescriptor
import java.io.IOException

class NetplaneVpnService : VpnService() {

    companion object {
        @Volatile
        private var instance: NetplaneVpnService? = null

        @Volatile
        private var vpnInterface: ParcelFileDescriptor? = null

        fun getInstance(): NetplaneVpnService? = instance

        fun getTunnelFd(): Int {
            return vpnInterface?.fd ?: -1
        }

        fun stopVpn() {
            try {
                vpnInterface?.close()
                vpnInterface = null
            } catch (e: IOException) {
                e.printStackTrace()
            }
        }
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
    }

    override fun onDestroy() {
        super.onDestroy()
        stopVpn()
        instance = null
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        return START_STICKY
    }

    fun createVpnInterface(address: String, routeAddress: String, prefixLength: Int): Int {
        try {
            // Stop any existing VPN
            stopVpn()

            val builder = Builder()
                .setSession("Netplane VPN")
                .addAddress(address, prefixLength)
                .addRoute(routeAddress, prefixLength)
                .setMtu(1400)
                .setBlocking(false)

            vpnInterface = builder.establish()
            return vpnInterface?.fd ?: -1
        } catch (e: Exception) {
            e.printStackTrace()
            return -1
        }
    }

    override fun onRevoke() {
        super.onRevoke()
        stopVpn()
    }
}

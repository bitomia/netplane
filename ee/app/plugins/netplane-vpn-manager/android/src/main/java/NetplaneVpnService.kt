package com.bitomia.netplane.vpn

import android.content.Intent
import android.net.VpnService
import android.os.ParcelFileDescriptor
import android.util.Log
import java.io.IOException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

class NetplaneVpnService : VpnService() {

    companion object {
        private const val TAG = "NetplaneVpnService"

        @Volatile
        private var instance: NetplaneVpnService? = null

        @Volatile
        private var vpnInterface: ParcelFileDescriptor? = null

        private var readyLatch = CountDownLatch(1)

        fun getInstance(): NetplaneVpnService? = instance

        fun waitForInstance(timeoutMs: Long): NetplaneVpnService? {
            Log.d(TAG, "Waiting for VPN service instance (timeout: ${timeoutMs}ms)")
            val ready = readyLatch.await(timeoutMs, TimeUnit.MILLISECONDS)
            Log.d(TAG, "Wait result: ready=$ready, instance=${instance != null}")
            return instance
        }

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
        Log.d(TAG, "onCreate called")
        instance = this
        readyLatch.countDown()
    }

    override fun onDestroy() {
        super.onDestroy()
        Log.d(TAG, "onDestroy called")
        stopVpn()
        instance = null
        readyLatch = CountDownLatch(1)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Log.d(TAG, "onStartCommand called")
        return START_STICKY
    }

    fun createVpnInterface(address: String, routeAddress: String, prefixLength: Int): Int {
        Log.d(TAG, "createVpnInterface: address=$address, route=$routeAddress, prefix=$prefixLength")
        try {
            // Stop any existing VPN
            stopVpn()

            val builder = Builder()
                .setSession("Netplane VPN")
                .addAddress(address, prefixLength)
                .addRoute(routeAddress, prefixLength)
                .setMtu(1400)
                .setBlocking(false)

            Log.d(TAG, "Calling establish()...")
            vpnInterface = builder.establish()

            if (vpnInterface == null) {
                Log.e(TAG, "establish() returned null - VPN not prepared or permission revoked")
                return -1
            }

            val fd = vpnInterface!!.fd
            Log.d(TAG, "VPN interface established, fd=$fd")
            return fd
        } catch (e: Exception) {
            Log.e(TAG, "createVpnInterface failed", e)
            return -1
        }
    }

    override fun onRevoke() {
        super.onRevoke()
        Log.d(TAG, "onRevoke called")
        stopVpn()
    }
}

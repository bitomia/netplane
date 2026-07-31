package com.example.netplane

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.ParcelFileDescriptor
import android.util.Log
import io.flutter.plugin.common.MethodChannel
import java.net.InetAddress

/**
 * Foreground [VpnService] that owns the TUN device for the netplane tunnel.
 *
 * Android only lets a [VpnService] create a TUN, and the tunnel IP must be known
 * *before* `establish()`. So the flow is split across the Dart/Rust bridge:
 *   1. Rust `prepareTunnel` runs the handshake and returns the assigned IP/netmask.
 *   2. This service is started with those params, builds the tunnel and hands the
 *      raw fd back to Dart (via [pendingResult]).
 *   3. Rust `connectFd` runs the packet loop on that fd, in-process.
 *
 * The [ParcelFileDescriptor] is kept alive here for the tunnel's lifetime and
 * closed on stop — the Rust side is told not to close it on drop.
 */
class NetplaneVpnService : VpnService() {
    private var pfd: ParcelFileDescriptor? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_ESTABLISH -> establish(intent)
            ACTION_STOP -> stop()
        }
        return START_NOT_STICKY
    }

    private fun establish(intent: Intent) {
        startForegroundNotification()

        val ipAddr = intent.getStringExtra(EXTRA_IP) ?: return failEstablish("missing ip")
        val netmask = intent.getStringExtra(EXTRA_NETMASK) ?: "255.255.255.255"
        val mtu = intent.getIntExtra(EXTRA_MTU, 1400)

        try {
            val prefix = prefixLength(netmask)
            val builder = Builder()
                .setSession("netplane")
                .setMtu(mtu)
                .addAddress(ipAddr, prefix)
                // Route only the overlay subnet through the tunnel; relay traffic
                // (public internet) stays on the default network, so the transport
                // socket doesn't need protecting.
                .addRoute(networkAddress(ipAddr, netmask), prefix)

            val descriptor = builder.establish()
                ?: return failEstablish("establish() returned null (permission revoked?)")
            pfd = descriptor
            deliverResult { it.success(descriptor.fd) }
        } catch (e: Exception) {
            Log.e(TAG, "establish failed", e)
            failEstablish(e.message ?: "establish failed")
        }
    }

    private fun stop() {
        try {
            pfd?.close()
        } catch (e: Exception) {
            Log.w(TAG, "closing tun fd failed", e)
        }
        pfd = null
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            stopForeground(STOP_FOREGROUND_REMOVE)
        } else {
            @Suppress("DEPRECATION")
            stopForeground(true)
        }
        stopSelf()
    }

    override fun onDestroy() {
        try {
            pfd?.close()
        } catch (_: Exception) {
        }
        pfd = null
        super.onDestroy()
    }

    override fun onRevoke() {
        // System or another VPN app revoked our tunnel.
        stop()
        super.onRevoke()
    }

    private fun failEstablish(message: String) {
        deliverResult { it.error("establish_failed", message, null) }
        stop()
    }

    private fun startForegroundNotification() {
        val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "VPN",
                NotificationManager.IMPORTANCE_LOW,
            )
            manager.createNotificationChannel(channel)
        }

        val openApp = packageManager.getLaunchIntentForPackage(packageName)
        val pending = openApp?.let {
            PendingIntent.getActivity(
                this,
                0,
                it,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
        }

        val notification: Notification = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
                .setContentTitle("netplane")
                .setContentText("VPN connected")
                .setSmallIcon(applicationInfo.icon)
                .setContentIntent(pending)
                .setOngoing(true)
                .build()
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
                .setContentTitle("netplane")
                .setContentText("VPN connected")
                .setSmallIcon(applicationInfo.icon)
                .setContentIntent(pending)
                .setOngoing(true)
                .build()
        }

        startForeground(NOTIFICATION_ID, notification)
    }

    companion object {
        private const val TAG = "NetplaneVpnService"
        private const val CHANNEL_ID = "netplane_vpn"
        private const val NOTIFICATION_ID = 1

        const val ACTION_ESTABLISH = "com.example.netplane.ESTABLISH"
        const val ACTION_STOP = "com.example.netplane.STOP"
        const val EXTRA_IP = "ipAddr"
        const val EXTRA_NETMASK = "netmask"
        const val EXTRA_MTU = "mtu"

        /**
         * The pending `establish` MethodChannel result, set by [MainActivity]
         * before the service is started and completed here once the fd is ready.
         */
        var pendingResult: MethodChannel.Result? = null

        private val mainHandler = Handler(Looper.getMainLooper())

        private fun deliverResult(block: (MethodChannel.Result) -> Unit) {
            val result = pendingResult ?: return
            pendingResult = null
            mainHandler.post { block(result) }
        }

        /** Convert a dotted-quad netmask (e.g. `255.255.255.0`) to a prefix length. */
        fun prefixLength(netmask: String): Int {
            val bytes = InetAddress.getByName(netmask).address
            var count = 0
            for (b in bytes) {
                count += Integer.bitCount(b.toInt() and 0xff)
            }
            return count
        }

        /** Network base address for `ip & netmask`, e.g. `10.0.0.5 / 255.255.255.0` -> `10.0.0.0`. */
        fun networkAddress(ip: String, netmask: String): String {
            val ipB = InetAddress.getByName(ip).address
            val maskB = InetAddress.getByName(netmask).address
            val out = ByteArray(ipB.size)
            for (i in ipB.indices) {
                out[i] = (ipB[i].toInt() and maskB[i].toInt()).toByte()
            }
            return InetAddress.getByAddress(out).hostAddress ?: ip
        }
    }
}

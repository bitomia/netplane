package com.example.netplane

import android.app.Activity
import android.content.Intent
import android.net.VpnService
import androidx.core.content.ContextCompat
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

/**
 * Hosts the `netplane/vpn/android` MethodChannel that drives [NetplaneVpnService].
 *
 *   prepare()  -> Bool   request VPN consent; true once granted
 *   establish(ipAddr, netmask, mtu) -> Int   start the tunnel, return the TUN fd
 *   stop()     -> Void   tear the tunnel down
 */
class MainActivity : FlutterActivity() {
    private var pendingPrepareResult: MethodChannel.Result? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "prepare" -> prepare(result)
                    "establish" -> establish(
                        call.argument("ipAddr"),
                        call.argument("netmask"),
                        call.argument("mtu"),
                        result,
                    )
                    "stop" -> stop(result)
                    else -> result.notImplemented()
                }
            }
    }

    private fun prepare(result: MethodChannel.Result) {
        val intent = VpnService.prepare(this)
        if (intent == null) {
            result.success(true)
            return
        }
        // Consent dialog needed; complete the result in onActivityResult.
        pendingPrepareResult = result
        startActivityForResult(intent, VPN_REQUEST_CODE)
    }

    private fun establish(
        ipAddr: String?,
        netmask: String?,
        mtu: Int?,
        result: MethodChannel.Result,
    ) {
        if (ipAddr == null) {
            result.error("bad_args", "ipAddr is required", null)
            return
        }
        NetplaneVpnService.pendingResult = result
        val intent = Intent(this, NetplaneVpnService::class.java).apply {
            action = NetplaneVpnService.ACTION_ESTABLISH
            putExtra(NetplaneVpnService.EXTRA_IP, ipAddr)
            putExtra(NetplaneVpnService.EXTRA_NETMASK, netmask ?: "255.255.255.255")
            putExtra(NetplaneVpnService.EXTRA_MTU, mtu ?: 1400)
        }
        ContextCompat.startForegroundService(this, intent)
    }

    private fun stop(result: MethodChannel.Result) {
        val intent = Intent(this, NetplaneVpnService::class.java).apply {
            action = NetplaneVpnService.ACTION_STOP
        }
        startService(intent)
        result.success(null)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        if (requestCode == VPN_REQUEST_CODE) {
            pendingPrepareResult?.success(resultCode == Activity.RESULT_OK)
            pendingPrepareResult = null
            return
        }
        super.onActivityResult(requestCode, resultCode, data)
    }

    companion object {
        private const val CHANNEL = "netplane/vpn/android"
        private const val VPN_REQUEST_CODE = 0x7654
    }
}

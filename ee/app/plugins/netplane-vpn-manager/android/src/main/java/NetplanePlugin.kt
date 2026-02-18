package com.bitomia.netplane.vpn

import android.app.Activity
import android.content.Intent
import android.net.VpnService
import android.util.Log
import androidx.activity.result.ActivityResult
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke

@InvokeArg
class StartVpnArgs {
    var address: String = ""
    var routeAddress: String = ""
    var prefixLength: Int = 24
}

@TauriPlugin
class NetplanePlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        private const val TAG = "NetplanePlugin"
    }

    private val vpnManager = NetplaneVpnManager(activity)

    @Command
    fun ping(invoke: Invoke) {
        val ret = JSObject()
        invoke.resolve(ret)
    }

    @Command
    fun requestVpnPermission(invoke: Invoke) {
        Log.d(TAG, "requestVpnPermission called")
        val intent = VpnService.prepare(activity)
        if (intent == null) {
            Log.d(TAG, "VPN permission already granted")
            val ret = JSObject()
            ret.put("granted", true)
            invoke.resolve(ret)
        } else {
            Log.d(TAG, "Launching VPN permission dialog")
            startActivityForResult(invoke, intent, "vpnPermissionResult")
        }
    }

    @ActivityCallback
    fun vpnPermissionResult(invoke: Invoke, result: ActivityResult) {
        val granted = result.resultCode == Activity.RESULT_OK
        Log.d(TAG, "VPN permission result: granted=$granted")
        val ret = JSObject()
        ret.put("granted", granted)
        invoke.resolve(ret)
    }

    @Command
    fun startVpn(invoke: Invoke) {
        Log.d(TAG, "startVpn called")
        val args = invoke.parseArgs(StartVpnArgs::class.java)
        Log.d(TAG, "startVpn args: address=${args.address}, route=${args.routeAddress}, prefix=${args.prefixLength}")
        vpnManager.startVpnService()

        Thread {
            val fd = vpnManager.createTunnelInterface(args.address, args.routeAddress, args.prefixLength)
            Log.d(TAG, "startVpn result: fd=$fd")
            val ret = JSObject()
            ret.put("fd", fd)
            invoke.resolve(ret)
        }.start()
    }

    @Command
    fun stopVpn(invoke: Invoke) {
        Log.d(TAG, "stopVpn called")
        vpnManager.stopVpnService()
        invoke.resolve(JSObject())
    }

    @Command
    fun getTunnelFd(invoke: Invoke) {
        val fd = NetplaneVpnService.getTunnelFd()
        Log.d(TAG, "getTunnelFd: fd=$fd")
        val ret = JSObject()
        ret.put("fd", fd)
        invoke.resolve(ret)
    }
}

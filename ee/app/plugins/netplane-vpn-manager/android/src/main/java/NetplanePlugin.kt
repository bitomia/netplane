package com.bitomia.netplane.vpn

import android.app.Activity
import android.content.Intent
import android.net.VpnService
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

    private val vpnManager = NetplaneVpnManager(activity)

    @Command
    fun ping(invoke: Invoke) {
        val ret = JSObject()
        invoke.resolve(ret)
    }

    @Command
    fun requestVpnPermission(invoke: Invoke) {
        val intent = VpnService.prepare(activity)
        if (intent == null) {
            val ret = JSObject()
            ret.put("granted", true)
            invoke.resolve(ret)
        } else {
            startActivityForResult(invoke, intent, "vpnPermissionResult")
        }
    }

    @ActivityCallback
    fun vpnPermissionResult(invoke: Invoke, result: ActivityResult) {
        val ret = JSObject()
        ret.put("granted", result.resultCode == Activity.RESULT_OK)
        invoke.resolve(ret)
    }

    @Command
    fun startVpn(invoke: Invoke) {
        val args = invoke.parseArgs(StartVpnArgs::class.java)
        vpnManager.startVpnService()

        // Wait briefly for service to bind
        android.os.Handler(activity.mainLooper).postDelayed({
            val fd = vpnManager.createTunnelInterface(args.address, args.routeAddress, args.prefixLength)
            val ret = JSObject()
            ret.put("fd", fd)
            invoke.resolve(ret)
        }, 500)
    }

    @Command
    fun stopVpn(invoke: Invoke) {
        vpnManager.stopVpnService()
        invoke.resolve(JSObject())
    }

    @Command
    fun getTunnelFd(invoke: Invoke) {
        val fd = NetplaneVpnService.getTunnelFd()
        val ret = JSObject()
        ret.put("fd", fd)
        invoke.resolve(ret)
    }
}

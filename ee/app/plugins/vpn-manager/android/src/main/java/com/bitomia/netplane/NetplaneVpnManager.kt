package com.bitomia.netplane

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.VpnService
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume

class NetplaneVpnManager(private val context: Context) {
    
    companion object {
        private const val VPN_REQUEST_CODE = 1001
        private var vpnPermissionResult: CompletableDeferred<Boolean>? = null
    }
    
    fun isVpnPermissionGranted(): Boolean {
        val intent = VpnService.prepare(context)
        return intent == null
    }
    
    suspend fun requestVpnPermission(): Boolean {
        if (isVpnPermissionGranted()) {
            return true
        }
        
        return suspendCancellableCoroutine { continuation ->
            val intent = VpnService.prepare(context)
            if (intent != null && context is Activity) {
                vpnPermissionResult = CompletableDeferred()
                context.startActivityForResult(intent, VPN_REQUEST_CODE)
                
                // Wait for result
                vpnPermissionResult?.invokeOnCompletion {
                    continuation.resume(vpnPermissionResult?.getCompleted() ?: false)
                }
            } else {
                continuation.resume(false)
            }
        }
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
    
    // Call this from MainActivity.onActivityResult
    fun handleVpnPermissionResult(requestCode: Int, resultCode: Int) {
        if (requestCode == VPN_REQUEST_CODE) {
            val granted = resultCode == Activity.RESULT_OK
            vpnPermissionResult?.complete(granted)
            vpnPermissionResult = null
        }
    }
}
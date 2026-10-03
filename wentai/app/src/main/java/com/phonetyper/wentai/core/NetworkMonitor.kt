package com.phonetyper.wentai.core

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * 网络可用性观测。**仅前台注册回调**（spec 4.1.5 / 5.6）：进入后台必须注销，
 * 避免后台产生任何周期性网络活动或阻止系统冻结进程。
 */
class NetworkMonitor(private val context: Context) : NetworkState {

    private val connectivityManager: ConnectivityManager? =
        context.getSystemService(ConnectivityManager::class.java)

    private val _available = MutableStateFlow(true)
    val available: StateFlow<Boolean> = _available.asStateFlow()

    /** 由不可用恢复为可用时发出一次事件，用于立即触发重连。 */
    private val _restored = MutableSharedFlow<Unit>(extraBufferCapacity = 4)
    override val restored: SharedFlow<Unit> = _restored.asSharedFlow()

    private var callback: ConnectivityManager.NetworkCallback? = null

    override fun start() {
        if (callback != null) return
        val cm = connectivityManager ?: return
        val cb = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                if (!_available.value) {
                    _available.value = true
                    _restored.tryEmit(Unit)
                }
            }

            override fun onLost(network: Network) {
                _available.value = false
            }
        }
        callback = cb
        runCatching { cm.registerDefaultNetworkCallback(cb) }
    }

    override fun stop() {
        val cb = callback ?: return
        callback = null
        runCatching { connectivityManager?.unregisterNetworkCallback(cb) }
    }
}

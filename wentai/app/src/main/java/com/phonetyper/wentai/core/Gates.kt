package com.phonetyper.wentai.core

import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow

/**
 * 前台判定接口：唯一权威信号来源，便于单测注入。
 *
 * 「后台静默、不保活」策略以该接口的值为准：仅前台允许连接、重连与网络监听。
 */
interface ForegroundState {
    val isForeground: StateFlow<Boolean>
    val events: SharedFlow<LifecycleEvent>
}

/**
 * 网络状态接口：仅前台注册回调；网络恢复事件用于立即触发重连。
 */
interface NetworkState {
    val restored: SharedFlow<Unit>
    fun start()
    fun stop()
}

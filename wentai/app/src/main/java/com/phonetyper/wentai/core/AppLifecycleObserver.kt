package com.phonetyper.wentai.core

import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow

enum class LifecycleEvent {
    FOREGROUND,
    BACKGROUND,
}

/**
 * 进程级前后台观测（基于 ProcessLifecycleOwner），是唯一的前台判定来源。
 *
 * 「后台静默、不保活」策略由此触发：进入后台取消重连调度与网络监听，
 * 不创建前台服务、不发送通知、不注册任何唤醒。
 */
class AppLifecycleObserver : DefaultLifecycleObserver, ForegroundState {

    private val _isForeground = MutableStateFlow(true)
    override val isForeground: StateFlow<Boolean> = _isForeground.asStateFlow()

    private val _events = MutableSharedFlow<LifecycleEvent>(extraBufferCapacity = 16)
    override val events: SharedFlow<LifecycleEvent> = _events.asSharedFlow()

    fun attach() {
        ProcessLifecycleOwner.get().lifecycle.addObserver(this)
    }

    fun detach() {
        ProcessLifecycleOwner.get().lifecycle.removeObserver(this)
    }

    override fun onStart(owner: LifecycleOwner) {
        _isForeground.value = true
        _events.tryEmit(LifecycleEvent.FOREGROUND)
    }

    override fun onStop(owner: LifecycleOwner) {
        _isForeground.value = false
        _events.tryEmit(LifecycleEvent.BACKGROUND)
    }
}

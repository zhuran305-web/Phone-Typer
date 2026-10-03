package com.phonetyper.wentai.domain

import com.phonetyper.wentai.core.ForegroundState
import com.phonetyper.wentai.core.LifecycleEvent
import com.phonetyper.wentai.core.NetworkState
import com.phonetyper.wentai.core.RingLogBuffer
import com.phonetyper.wentai.data.ConfigApi
import com.phonetyper.wentai.data.ConfigFetchResult
import com.phonetyper.wentai.data.Inbound
import com.phonetyper.wentai.data.InboundParser
import com.phonetyper.wentai.data.Transport
import com.phonetyper.wentai.data.TransportEvent
import com.phonetyper.wentai.data.store.PairStore
import com.phonetyper.wentai.domain.model.ConnState
import com.phonetyper.wentai.domain.model.ErrorCode
import com.phonetyper.wentai.domain.model.OutboundMessage
import com.phonetyper.wentai.domain.model.OutboundType
import com.phonetyper.wentai.domain.model.PairInfo
import com.phonetyper.wentai.domain.model.SyncAck
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.launch

/** 连接层对外事件，供界面与同步协调器消费。 */
sealed interface ConnectionEvent {
    /** 连接建立：需清空输入框以对齐电脑端基线（spec 5.2.1-5）。 */
    data object Opened : ConnectionEvent
    data class Ack(val ack: SyncAck) : ConnectionEvent
    data class Clip(val text: String) : ConnectionEvent
    data object BadPin : ConnectionEvent
    data object BadJson : ConnectionEvent
}

/**
 * 连接管理器：`StateFlow<ConnState>` 为界面唯一状态源。
 *
 * 后台静默、不保活：进入后台仅取消重连计时与网络监听，不创建前台服务/通知/唤醒。
 * 可靠性、重连等行为仅在前台生效。
 */
class ConnectionManager(
    private val pairStore: PairStore,
    private val transport: Transport,
    private val lifecycle: ForegroundState,
    private val networkMonitor: NetworkState,
    private val configApi: ConfigApi,
    private val log: RingLogBuffer,
    private val scope: CoroutineScope,
) {

    private val _state = MutableStateFlow<ConnState>(ConnState.Unpaired)
    val state: StateFlow<ConnState> = _state.asStateFlow()

    private val _peerVersion = MutableStateFlow<String?>(null)
    val peerVersion: StateFlow<String?> = _peerVersion.asStateFlow()

    private val _events = MutableSharedFlow<ConnectionEvent>(extraBufferCapacity = 64)
    val events: SharedFlow<ConnectionEvent> = _events.asSharedFlow()

    @Volatile
    private var currentPairing: PairInfo? = null

    private var retryJob: Job? = null
    private var attempt: Int = 0

    private val isForeground: Boolean get() = lifecycle.isForeground.value

    init {
        scope.launch { pairStore.pairing.collect(::onPairingChanged) }
        scope.launch { transport.events.collect(::onTransportEvent) }
        scope.launch { lifecycle.events.collect(::onLifecycleEvent) }
        scope.launch {
            networkMonitor.restored.collect { if (isForeground) connectNow() }
        }
    }

    /** 当前配对信息（供同步协调器读取）。 */
    fun currentPairing(): PairInfo? = currentPairing

    /** 前台启动连接与重连调度。 */
    fun start() = onEnterForeground()

    fun onEnterForeground() {
        if (currentPairing == null) {
            setState(ConnState.Unpaired)
            return
        }
        networkMonitor.start()
        if (_state.value == ConnState.PinError) return
        connectNow()
    }

    fun onEnterBackground() {
        log.record(RingLogBuffer.EVENT_BACKGROUND)
        cancelRetry()
        networkMonitor.stop()
        // 仅取消调度，不主动断开、不新增任何保活手段
    }

    private fun onLifecycleEvent(event: LifecycleEvent) {
        when (event) {
            LifecycleEvent.FOREGROUND -> onEnterForeground()
            LifecycleEvent.BACKGROUND -> onEnterBackground()
        }
    }

    /** 用户手动重试（例如修正 PIN 之后）。 */
    fun retryNow() {
        if (currentPairing == null) {
            setState(ConnState.Unpaired)
            return
        }
        if (!isForeground) return
        attempt = 0
        cancelRetry()
        connectNow()
    }

    fun resetPairing() {
        scope.launch { pairStore.clear() }
    }

    suspend fun send(type: OutboundType, text: String): Result<Unit> {
        val info = currentPairing ?: return Result.failure(IllegalStateException("未配对"))
        if (_state.value != ConnState.Connected) {
            return Result.failure(IllegalStateException("未连接"))
        }
        return transport.send(OutboundMessage(pin = info.pin, type = type.wire, text = text))
    }

    // ---- 内部 ----

    private fun onPairingChanged(info: PairInfo?) {
        if (info == null) {
            currentPairing = null
            cancelRetry()
            transport.close()
            _peerVersion.value = null
            setState(ConnState.Unpaired)
            return
        }
        val changed = currentPairing != info
        currentPairing = info
        if (changed && isForeground) {
            attempt = 0
            connectNow()
        }
    }

    private fun connectNow() {
        val info = currentPairing
        if (info == null) {
            setState(ConnState.Unpaired)
            return
        }
        if (!isForeground) return
        val current = _state.value
        if (current == ConnState.Connecting) return
        if (current == ConnState.Connected && transport.isOpen) return

        cancelRetry()
        setState(ConnState.Connecting)
        log.record(RingLogBuffer.EVENT_CONNECTING)
        scope.launch { transport.connect(info.host, info.wsPort) }
    }

    private fun onTransportEvent(event: TransportEvent) {
        when (event) {
            is TransportEvent.Open -> onOpen()
            is TransportEvent.Message -> onMessage(event.text)
            is TransportEvent.Closed -> onDisconnected()
            is TransportEvent.Failure -> onDisconnected()
        }
    }

    private fun onOpen() {
        attempt = 0
        setState(ConnState.Connected)
        log.record(RingLogBuffer.EVENT_CONNECTED)
        if (isForeground) networkMonitor.start()
        _events.tryEmit(ConnectionEvent.Opened)
        currentPairing?.let { info ->
            scope.launch {
                when (val result = configApi.fetch(info.host, info.httpPort)) {
                    is ConfigFetchResult.Success -> _peerVersion.value = result.config.version
                    is ConfigFetchResult.Failure -> Unit
                }
            }
        }
    }

    private fun onMessage(raw: String) {
        when (val inbound = InboundParser.parse(raw)) {
            is Inbound.Ack -> _events.tryEmit(ConnectionEvent.Ack(inbound.ack))
            is Inbound.Clip -> _events.tryEmit(ConnectionEvent.Clip(inbound.text))
            is Inbound.Error -> {
                if (inbound.code == ErrorCode.BAD_PIN) {
                    onBadPin()
                } else {
                    // bad-json：忽略，状态不变、不重发
                    _events.tryEmit(ConnectionEvent.BadJson)
                }
            }
            Inbound.Unknown -> Unit
        }
    }

    private fun onBadPin() {
        cancelRetry()
        setState(ConnState.PinError)
        log.record(RingLogBuffer.EVENT_PIN_REJECTED)
        _events.tryEmit(ConnectionEvent.BadPin)
    }

    private fun onDisconnected() {
        val current = _state.value
        if (current == ConnState.PinError || current == ConnState.Unpaired) return
        if (!isForeground) return
        setState(ConnState.Reconnecting)
        log.record(RingLogBuffer.EVENT_DISCONNECTED)
        scheduleRetry()
    }

    private fun scheduleRetry() {
        cancelRetry()
        val delayMs = backoffDelay(attempt)
        attempt += 1
        retryJob = scope.launch {
            delay(delayMs)
            if (_state.value == ConnState.PinError || !isForeground) return@launch
            connectNow()
        }
    }

    private fun backoffDelay(round: Int): Long {
        val base = BASE_BACKOFF_MS * (round + 1)
        return base.coerceAtMost(MAX_BACKOFF_MS)
    }

    private fun cancelRetry() {
        retryJob?.cancel()
        retryJob = null
    }

    private fun setState(next: ConnState) {
        if (_state.value != next) _state.value = next
    }

    companion object {
        const val BASE_BACKOFF_MS = 3000L
        const val MAX_BACKOFF_MS = 30000L
    }
}

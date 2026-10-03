package com.phonetyper.wentai.testutil

import com.phonetyper.wentai.core.ForegroundState
import com.phonetyper.wentai.core.LifecycleEvent
import com.phonetyper.wentai.core.NetworkState
import com.phonetyper.wentai.data.Transport
import com.phonetyper.wentai.data.TransportEvent
import com.phonetyper.wentai.data.store.PairStore
import com.phonetyper.wentai.domain.model.OutboundMessage
import com.phonetyper.wentai.domain.model.PairInfo
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow

class FakePairStore : PairStore {
    private val state = MutableStateFlow<PairInfo?>(null)
    override val pairing: Flow<PairInfo?> = state.asStateFlow()
    override suspend fun save(info: PairInfo) {
        state.value = info
    }

    override suspend fun clear() {
        state.value = null
    }

    fun current(): PairInfo? = state.value
}

class FakeForegroundState : ForegroundState {
    private val _isForeground = MutableStateFlow(true)
    override val isForeground: StateFlow<Boolean> = _isForeground.asStateFlow()

    private val _events = MutableSharedFlow<LifecycleEvent>(extraBufferCapacity = 8)
    override val events: SharedFlow<LifecycleEvent> = _events.asSharedFlow()

    fun setForeground(value: Boolean) {
        _isForeground.value = value
        _events.tryEmit(if (value) LifecycleEvent.FOREGROUND else LifecycleEvent.BACKGROUND)
    }
}

class FakeNetworkState : NetworkState {
    private val _restored = MutableSharedFlow<Unit>(extraBufferCapacity = 4)
    override val restored: SharedFlow<Unit> = _restored.asSharedFlow()

    var started: Boolean = false
        private set

    override fun start() {
        started = true
    }

    override fun stop() {
        started = false
    }

    fun emitRestored() {
        _restored.tryEmit(Unit)
    }
}

class FakeTransport : Transport {
    private val _events = MutableSharedFlow<TransportEvent>(extraBufferCapacity = 16)
    override val events: Flow<TransportEvent> = _events.asSharedFlow()

    override var isOpen: Boolean = false
        private set

    var lastConnect: Pair<String, Int>? = null
    var connectCalls: Int = 0
    val sent = mutableListOf<OutboundMessage>()
    var sendResult: Result<Unit> = Result.success(Unit)

    override suspend fun connect(host: String, wsPort: Int): Result<Unit> {
        connectCalls++
        lastConnect = host to wsPort
        return Result.success(Unit)
    }

    override suspend fun send(message: OutboundMessage): Result<Unit> {
        sent += message
        return sendResult
    }

    override fun close() {
        isOpen = false
    }

    fun emitOpen() {
        isOpen = true
        _events.tryEmit(TransportEvent.Open)
    }

    fun emitMessage(text: String) {
        _events.tryEmit(TransportEvent.Message(text))
    }

    fun emitClosed() {
        isOpen = false
        _events.tryEmit(TransportEvent.Closed(null))
    }

    fun emitFailure() {
        isOpen = false
        _events.tryEmit(TransportEvent.Failure(null))
    }
}

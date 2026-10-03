package com.phonetyper.wentai.data

import com.phonetyper.wentai.domain.model.OutboundMessage
import kotlinx.coroutines.flow.Flow

/** 传输层事件。 */
sealed interface TransportEvent {
    data object Open : TransportEvent
    data class Message(val text: String) : TransportEvent
    data class Closed(val reason: String?) : TransportEvent
    data class Failure(val error: Throwable?) : TransportEvent
}

/**
 * 传输抽象：便于单测注入假实现，并隔离 OkHttp 细节。
 */
interface Transport {
    val events: Flow<TransportEvent>
    val isOpen: Boolean
    suspend fun connect(host: String, wsPort: Int): Result<Unit>
    suspend fun send(message: OutboundMessage): Result<Unit>
    fun close()
}

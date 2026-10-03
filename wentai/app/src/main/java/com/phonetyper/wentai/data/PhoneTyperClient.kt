package com.phonetyper.wentai.data

import com.phonetyper.wentai.domain.model.OutboundMessage
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.serialization.json.Json
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener

/**
 * OkHttp WebSocket 实现的传输层。
 *
 * - 上行报文严格为 `{"pin":..,"type":"sync"|"send","text":..}`（`server.py:279-332`）。
 * - 不发送应用层心跳/保活帧（`pingInterval = 0`），符合「后台静默、不保活」。
 * - 使用连接代次（token）过滤旧连接回调，避免重连时的状态抖动。
 */
class PhoneTyperClient(
    private val client: OkHttpClient = defaultClient(),
) : Transport {

    private val _events = MutableSharedFlow<TransportEvent>(extraBufferCapacity = 64)
    override val events: Flow<TransportEvent> = _events.asSharedFlow()

    private val generation = AtomicInteger(0)

    @Volatile
    private var socket: WebSocket? = null

    @Volatile
    private var open: Boolean = false

    override val isOpen: Boolean get() = open

    private val json = Json {
        encodeDefaults = true
        ignoreUnknownKeys = true
    }

    override suspend fun connect(host: String, wsPort: Int): Result<Unit> {
        closeQuietly()
        val token = generation.incrementAndGet()

        val request = Request.Builder()
            .url("ws://$host:$wsPort")
            .build()

        val listener = object : WebSocketListener() {
            private fun active(): Boolean = generation.get() == token

            override fun onOpen(webSocket: WebSocket, response: Response) {
                if (!active()) return
                open = true
                _events.tryEmit(TransportEvent.Open)
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                if (!active()) return
                _events.tryEmit(TransportEvent.Message(text))
            }

            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                webSocket.close(NORMAL_CLOSURE, null)
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                if (!active()) return
                open = false
                socket = null
                _events.tryEmit(TransportEvent.Closed(reason))
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                if (!active()) return
                open = false
                socket = null
                _events.tryEmit(TransportEvent.Failure(t))
            }
        }

        return runCatching {
            socket = client.newWebSocket(request, listener)
            Unit
        }
    }

    override suspend fun send(message: OutboundMessage): Result<Unit> = runCatching {
        val ws = socket ?: error("WebSocket 未连接")
        val payload = json.encodeToString(OutboundMessage.serializer(), message)
        if (!ws.send(payload)) error("发送失败")
    }

    override fun close() {
        closeQuietly()
    }

    private fun closeQuietly() {
        generation.incrementAndGet()
        val ws = socket
        socket = null
        open = false
        runCatching { ws?.close(NORMAL_CLOSURE, "bye") }
    }

    companion object {
        private const val NORMAL_CLOSURE = 1000
        private const val CONNECT_TIMEOUT_MS = 3000L

        fun defaultClient(): OkHttpClient = OkHttpClient.Builder()
            .connectTimeout(CONNECT_TIMEOUT_MS, TimeUnit.MILLISECONDS)
            .readTimeout(0, TimeUnit.MILLISECONDS)
            .pingInterval(0, TimeUnit.MILLISECONDS)
            .retryOnConnectionFailure(true)
            .build()
    }
}

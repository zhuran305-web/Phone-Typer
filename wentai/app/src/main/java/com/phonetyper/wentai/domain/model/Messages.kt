package com.phonetyper.wentai.domain.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/** 上行文本类型，字段值与 `server.py` 严格一致。 */
enum class OutboundType(val wire: String) {
    SYNC("sync"),
    SEND("send"),
}

/** 上行报文：`{"pin":..,"type":"sync"|"send","text":..}`。 */
@Serializable
data class OutboundMessage(
    val pin: String,
    val type: String,
    val text: String,
)

/** 电脑端配置响应：`GET /api/config`。 */
@Serializable
data class ServerConfig(
    @SerialName("ws_port") val wsPort: Int = PairInfo.DEFAULT_WS_PORT,
    val version: String = "",
)

/** 下行同步回执：`{"ok":true,"bs":..,"typed":..,"s":..}`。 */
@Serializable
data class SyncAck(
    val ok: Boolean = false,
    val bs: Int = 0,
    val typed: Int = 0,
    val s: Int = 0,
)

/** 下行错误：`{"ok":false,"err":"bad-pin"|"bad-json"}`。 */
@Serializable
data class ServerError(
    val ok: Boolean = false,
    val err: String = "",
)

/** 下行剪贴板推送：`{"type":"clip","text":..}`。 */
@Serializable
data class ClipMessage(
    val type: String = "",
    val text: String = "",
)

/** 服务端错误码。 */
enum class ErrorCode(val wire: String) {
    BAD_PIN("bad-pin"),
    BAD_JSON("bad-json"),
    ;

    companion object {
        fun fromWire(value: String): ErrorCode? = entries.firstOrNull { it.wire == value }
    }
}

/** 会话级接收消息，仅内存，进程结束即销毁。 */
data class InboxMessage(
    val text: String,
    val receivedAt: Long,
)

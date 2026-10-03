package com.phonetyper.wentai.data

import com.phonetyper.wentai.domain.model.ErrorCode
import com.phonetyper.wentai.domain.model.SyncAck
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonPrimitive

/** 下行报文的结构化结果。 */
sealed interface Inbound {
    data class Ack(val ack: SyncAck) : Inbound
    data class Error(val code: ErrorCode) : Inbound
    data class Clip(val text: String) : Inbound

    /** 未知结构，按忽略处理（spec 5.2.3-3）。 */
    data object Unknown : Inbound
}

/** 下行反序列化：`{"ok":..}` / `{"ok":false,"err":..}` / `{"type":"clip","text":..}`。 */
object InboundParser {

    private const val TYPE_CLIP = "clip"

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
    }

    fun parse(raw: String): Inbound = runCatching { doParse(raw) }.getOrDefault(Inbound.Unknown)

    private fun doParse(raw: String): Inbound {
        val obj = json.parseToJsonElement(raw) as? JsonObject ?: return Inbound.Unknown

        val type = obj["type"]?.let { runCatching { it.jsonPrimitive.contentOrNull }.getOrNull() }
        if (type == TYPE_CLIP) {
            val text = obj["text"]
                ?.let { runCatching { it.jsonPrimitive.contentOrNull }.getOrNull() }
                .orEmpty()
            return Inbound.Clip(text)
        }

        val ok = obj["ok"]?.let { runCatching { it.jsonPrimitive.booleanOrNull }.getOrNull() }
        return when (ok) {
            true -> Inbound.Ack(
                SyncAck(
                    ok = true,
                    bs = intField(obj, "bs"),
                    typed = intField(obj, "typed"),
                    s = intField(obj, "s"),
                )
            )

            false -> {
                val err = obj["err"]
                    ?.let { runCatching { it.jsonPrimitive.contentOrNull }.getOrNull() }
                    .orEmpty()
                ErrorCode.fromWire(err)?.let { Inbound.Error(it) } ?: Inbound.Unknown
            }

            null -> Inbound.Unknown
        }
    }

    private fun intField(obj: JsonObject, key: String): Int =
        obj[key]?.let { runCatching { it.jsonPrimitive.intOrNull }.getOrNull() } ?: 0
}

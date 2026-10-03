package com.phonetyper.wentai.data

import com.phonetyper.wentai.domain.model.PairInfo
import com.phonetyper.wentai.domain.model.PairingError
import java.net.URI
import java.net.URLDecoder

/** 配对 URI 解析结果。`wsPort` 为 null 表示需要经 `/api/config` 补全。 */
sealed interface PairParseResult {
    data class Ok(
        val host: String,
        val httpPort: Int,
        val wsPort: Int?,
        val pin: String,
    ) : PairParseResult

    data class Error(val error: PairingError) : PairParseResult
}

/**
 * 解析 `ptyper://connect?ip=&http=&ws=&pin=`（`tray.py:68-69` 生成的格式）。
 *
 * 兼容 `ws` 缺失（spec 4.5.3）：此时返回 Ok 且 wsPort=null，交由上层补全；
 * 非 ptyper 前缀或缺 ip/pin 直接判失败，调用方保留原配置不变。
 */
class PairUriParser {

    fun parse(raw: String): PairParseResult {
        val trimmed = raw.trim()
        if (trimmed.isEmpty()) return error(PairingError.InvalidFormat)

        val uri = runCatching { URI(trimmed) }.getOrNull()
            ?: return error(PairingError.InvalidFormat)

        if (!uri.scheme.equals(SCHEME, ignoreCase = true)) {
            return error(PairingError.InvalidFormat)
        }
        val target = uri.host ?: uri.authority
        if (!HOST.equals(target, ignoreCase = true)) {
            return error(PairingError.InvalidFormat)
        }

        val params = parseQuery(uri.rawQuery)

        val host = params["ip"]?.trim().orEmpty()
        if (host.isEmpty()) return error(PairingError.Incomplete)

        val pin = params["pin"]?.trim().orEmpty()
        if (pin.isEmpty()) return error(PairingError.Incomplete)

        val httpRaw = params["http"]
        val httpPort = if (httpRaw.isNullOrBlank()) {
            PairInfo.DEFAULT_HTTP_PORT
        } else {
            parsePort(httpRaw) ?: return error(PairingError.InvalidFormat)
        }

        val wsRaw = params["ws"]
        val wsPort = if (wsRaw.isNullOrBlank()) {
            null
        } else {
            parsePort(wsRaw) ?: return error(PairingError.InvalidFormat)
        }

        return PairParseResult.Ok(
            host = host,
            httpPort = httpPort,
            wsPort = wsPort,
            pin = pin,
        )
    }

    private fun parseQuery(rawQuery: String?): Map<String, String> {
        if (rawQuery.isNullOrBlank()) return emptyMap()
        val map = LinkedHashMap<String, String>()
        for (segment in rawQuery.split("&")) {
            if (segment.isEmpty()) continue
            val index = segment.indexOf('=')
            if (index < 0) continue
            val key = decode(segment.substring(0, index))
            val value = decode(segment.substring(index + 1))
            map[key] = value
        }
        return map
    }

    private fun decode(value: String): String =
        runCatching { URLDecoder.decode(value, "UTF-8") }.getOrDefault(value)

    private fun parsePort(value: String): Int? {
        val port = value.trim().toIntOrNull() ?: return null
        return if (port in 1..65535) port else null
    }

    private fun error(err: PairingError): PairParseResult.Error = PairParseResult.Error(err)

    companion object {
        private const val SCHEME = "ptyper"
        private const val HOST = "connect"
    }
}

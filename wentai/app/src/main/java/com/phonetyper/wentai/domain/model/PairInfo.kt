package com.phonetyper.wentai.domain.model

import kotlinx.serialization.Serializable

/** 配对信息来源。 */
@Serializable
enum class PairSource {
    SCAN,
    GALLERY,
    MANUAL,
}

/**
 * 结构化配对信息，仅存本机。
 *
 * 默认端口与 `config.json` 对齐；旧配置字段缺失时按默认值兜底，不抛异常。
 */
@Serializable
data class PairInfo(
    val host: String,
    val httpPort: Int = DEFAULT_HTTP_PORT,
    val wsPort: Int = DEFAULT_WS_PORT,
    val pin: String,
    val source: PairSource = PairSource.MANUAL,
    val lastUsedAt: Long = 0L,
) {
    companion object {
        const val DEFAULT_HTTP_PORT = 8766
        const val DEFAULT_WS_PORT = 8767
    }
}

/** 手动录入的配对参数。 */
data class ManualPairInput(
    val host: String,
    val httpPort: Int,
    val wsPort: Int,
    val pin: String,
)

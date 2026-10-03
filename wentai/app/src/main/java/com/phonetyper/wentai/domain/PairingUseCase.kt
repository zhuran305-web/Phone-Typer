package com.phonetyper.wentai.domain

import com.phonetyper.wentai.data.ConfigApi
import com.phonetyper.wentai.data.ConfigFetchResult
import com.phonetyper.wentai.data.PairParseResult
import com.phonetyper.wentai.data.PairUriParser
import com.phonetyper.wentai.data.store.PairStore
import com.phonetyper.wentai.domain.model.ManualPairInput
import com.phonetyper.wentai.domain.model.PairInfo
import com.phonetyper.wentai.domain.model.PairSource
import com.phonetyper.wentai.domain.model.PairingError
import com.phonetyper.wentai.domain.model.PairingResult

/**
 * 配对用例编排：扫码 / 相册 / 手动三入口 → 解析 → 端口补全 → 持久化。
 *
 * 硬约束：任一环节失败时**不改动现有配置**（spec 5.1.3-1）；
 * host 或 pin 为空时不发起任何网络请求（spec 5.1.1-7/8）。
 */
class PairingUseCase(
    private val parser: PairUriParser,
    private val configApi: ConfigApi,
    private val pairStore: PairStore,
    private val clock: () -> Long = { System.currentTimeMillis() },
) {

    suspend fun applyFromScanned(raw: String): PairingResult = applyRaw(raw, PairSource.SCAN)

    /** 相册识码：入参为已从图片解码出的二维码文本。 */
    suspend fun applyFromImage(decoded: String): PairingResult = applyRaw(decoded, PairSource.GALLERY)

    suspend fun applyManual(input: ManualPairInput): PairingResult {
        val host = input.host.trim()
        val pin = input.pin.trim()
        if (host.isEmpty() || pin.isEmpty()) {
            return PairingResult.Failure(PairingError.InvalidManual)
        }
        if (input.httpPort !in 1..65535 || input.wsPort !in 1..65535) {
            return PairingResult.Failure(PairingError.InvalidManual)
        }
        val info = buildInfo(
            host = host,
            httpPort = input.httpPort,
            wsPort = input.wsPort,
            pin = pin,
            source = PairSource.MANUAL,
        )
        pairStore.save(info)
        return PairingResult.Success(info)
    }

    private suspend fun applyRaw(raw: String, source: PairSource): PairingResult {
        val parsed = when (val result = parser.parse(raw)) {
            is PairParseResult.Error -> return PairingResult.Failure(result.error)
            is PairParseResult.Ok -> result
        }

        val wsPort = parsed.wsPort ?: run {
            when (val fetched = configApi.fetch(parsed.host, parsed.httpPort)) {
                is ConfigFetchResult.Failure -> return PairingResult.Failure(fetched.error)
                is ConfigFetchResult.Success -> fetched.config.wsPort
            }
        }

        val info = buildInfo(
            host = parsed.host,
            httpPort = parsed.httpPort,
            wsPort = wsPort,
            pin = parsed.pin,
            source = source,
        )
        pairStore.save(info)
        return PairingResult.Success(info)
    }

    private fun buildInfo(
        host: String,
        httpPort: Int,
        wsPort: Int,
        pin: String,
        source: PairSource,
    ) = PairInfo(
        host = host,
        httpPort = httpPort,
        wsPort = wsPort,
        pin = pin,
        source = source,
        lastUsedAt = clock(),
    )
}

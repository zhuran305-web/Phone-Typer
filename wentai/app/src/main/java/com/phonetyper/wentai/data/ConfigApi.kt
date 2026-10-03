package com.phonetyper.wentai.data

import com.phonetyper.wentai.domain.model.PairingError
import com.phonetyper.wentai.domain.model.ServerConfig
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit

/** 配置查询结果。 */
sealed interface ConfigFetchResult {
    data class Success(val config: ServerConfig) : ConfigFetchResult
    data class Failure(val error: PairingError) : ConfigFetchResult
}

/**
 * 配置查询接口：`GET http://{ip}:{httpPort}/api/config`（`server.py:222-226`）。
 *
 * 仅用于「配对信息缺 ws 端口时补全」与「电脑端版本对照展示」，无副作用。
 */
class ConfigApi(
    private val client: OkHttpClient = defaultClient(),
) {

    suspend fun fetch(host: String, httpPort: Int): ConfigFetchResult = withContext(Dispatchers.IO) {
        val request = Request.Builder()
            .url("http://$host:$httpPort/api/config")
            .get()
            .build()

        try {
            client.newCall(request).execute().use { response ->
                val body = response.body?.string().orEmpty()
                if (!response.isSuccessful || body.isBlank()) {
                    return@withContext ConfigFetchResult.Failure(PairingError.InvalidResponse)
                }
                val config = runCatching { json.decodeFromString(ServerConfig.serializer(), body) }
                    .getOrNull()
                if (config == null) {
                    ConfigFetchResult.Failure(PairingError.InvalidResponse)
                } else {
                    ConfigFetchResult.Success(config)
                }
            }
        } catch (_: Exception) {
            ConfigFetchResult.Failure(PairingError.ServerUnreachable)
        }
    }

    companion object {
        private const val CONNECT_TIMEOUT_MS = 3000L
        private const val READ_TIMEOUT_MS = 3000L

        private val json = kotlinx.serialization.json.Json {
            ignoreUnknownKeys = true
            isLenient = true
        }

        fun defaultClient(): OkHttpClient = OkHttpClient.Builder()
            .connectTimeout(CONNECT_TIMEOUT_MS, TimeUnit.MILLISECONDS)
            .readTimeout(READ_TIMEOUT_MS, TimeUnit.MILLISECONDS)
            .build()
    }
}

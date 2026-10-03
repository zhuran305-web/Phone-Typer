package com.phonetyper.wentai.testutil

import com.phonetyper.wentai.data.store.SettingsStore
import com.phonetyper.wentai.domain.model.WorkMode
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import okhttp3.OkHttpClient
import java.util.concurrent.TimeUnit

class FakeSettingsStore : SettingsStore {
    private val state = MutableStateFlow(WorkMode.DEFAULT)
    override val workMode: Flow<WorkMode> = state.asStateFlow()
    override suspend fun setWorkMode(mode: WorkMode) {
        state.value = mode
    }
}

/** 短超时客户端：让测试中的不可达连接快速失败。 */
fun shortTimeoutClient(): OkHttpClient = OkHttpClient.Builder()
    .connectTimeout(200, TimeUnit.MILLISECONDS)
    .readTimeout(200, TimeUnit.MILLISECONDS)
    .build()

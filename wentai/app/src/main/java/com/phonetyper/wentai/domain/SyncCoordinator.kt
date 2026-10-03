package com.phonetyper.wentai.domain

import com.phonetyper.wentai.data.store.SettingsStore
import com.phonetyper.wentai.domain.model.OutboundType
import com.phonetyper.wentai.domain.model.WorkMode
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * 同步协调器：组合态判定、150ms 防抖合并、全文上行、连接/模式切换时对齐。
 *
 * 规则（spec 5.2.1）：
 * - 组合态（输入法拼写中）期间不上行；
 * - 仅在前台、已连接、实时同步模式下发送；
 * - 只发送输入框全文，不发送增量；
 * - 150ms 静默窗口合并连续输入，端到端延迟控制在 200ms 内。
 */
class SyncCoordinator(
    private val connectionManager: ConnectionManager,
    private val settingsStore: SettingsStore,
    private val scope: CoroutineScope,
    private val debounceMs: Long = DEFAULT_DEBOUNCE_MS,
) {

    private var debounceJob: Job? = null

    @Volatile
    private var composing: Boolean = false

    @Volatile
    private var mode: WorkMode = WorkMode.DEFAULT

    fun setMode(workMode: WorkMode) {
        mode = workMode
        cancelDebounce()
        composing = false
    }

    /**
     * 输入框内容变化入口。
     * @param composing 输入法是否处于组合态（拼写未上屏）。
     */
    fun onEditorChanged(text: String, composing: Boolean) {
        this.composing = composing
        if (composing) {
            // 组合态期间取消待发同步
            cancelDebounce()
            return
        }
        if (mode != WorkMode.LIVE_SYNC) {
            cancelDebounce()
            return
        }
        scheduleDebounce(text)
    }

    /** 回前台或连接建立后请求一次全文对齐。 */
    fun flushAlign(text: String) {
        cancelDebounce()
        if (mode != WorkMode.LIVE_SYNC) return
        scope.launch { connectionManager.send(OutboundType.SYNC, text) }
    }

    /** 整段发送；空内容拦截，成功与否由调用方决定是否清空输入框。 */
    suspend fun sendSegment(text: String): Result<Unit> {
        if (text.isEmpty()) {
            return Result.failure(IllegalArgumentException("内容为空"))
        }
        return connectionManager.send(OutboundType.SEND, text)
    }

    /** 清空并触发电脑端回删：实时模式下立即发送一次空串 sync。 */
    fun clearWithRollback() {
        cancelDebounce()
        if (mode != WorkMode.LIVE_SYNC) return
        scope.launch { connectionManager.send(OutboundType.SYNC, "") }
    }

    private fun scheduleDebounce(text: String) {
        cancelDebounce()
        debounceJob = scope.launch {
            delay(debounceMs)
            if (composing) return@launch
            if (mode != WorkMode.LIVE_SYNC) return@launch
            connectionManager.send(OutboundType.SYNC, text)
        }
    }

    private fun cancelDebounce() {
        debounceJob?.cancel()
        debounceJob = null
    }

    companion object {
        const val DEFAULT_DEBOUNCE_MS = 150L
    }
}

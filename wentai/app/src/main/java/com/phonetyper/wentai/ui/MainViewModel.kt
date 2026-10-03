package com.phonetyper.wentai.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.phonetyper.wentai.BuildConfig
import com.phonetyper.wentai.core.ClipboardWriter
import com.phonetyper.wentai.core.ForegroundState
import com.phonetyper.wentai.core.LifecycleEvent
import com.phonetyper.wentai.data.store.SettingsStore
import com.phonetyper.wentai.domain.ConnectionEvent
import com.phonetyper.wentai.domain.ConnectionManager
import com.phonetyper.wentai.domain.SyncCoordinator
import com.phonetyper.wentai.domain.model.ConnState
import com.phonetyper.wentai.domain.model.WorkMode
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * 主界面 ViewModel：以单一 `StateFlow<MainUiState>` 对外暴露，状态只能来自此层。
 */
class MainViewModel(
    private val connectionManager: ConnectionManager,
    private val syncCoordinator: SyncCoordinator,
    private val settingsStore: SettingsStore,
    private val clipboardWriter: ClipboardWriter,
    private val lifecycleObserver: ForegroundState,
) : ViewModel() {

    private val _ui = MutableStateFlow(MainUiState(appVersion = BuildConfig.VERSION_NAME))
    val ui: StateFlow<MainUiState> = _ui.asStateFlow()

    /** 一次性请求界面清空输入框（连接建立/模式切换/清空/发送成功后）。 */
    private val _clearEditor = MutableSharedFlow<Unit>(extraBufferCapacity = 4)
    val clearEditor: SharedFlow<Unit> = _clearEditor.asSharedFlow()

    init {
        observe()
    }

    fun start() {
        connectionManager.start()
    }

    fun onEditorChanged(text: String, composing: Boolean) {
        syncCoordinator.onEditorChanged(text, composing)
        _ui.update {
            it.copy(
                editorText = text,
                charCount = text.length,
                canSendSegment = canSend(it.workMode, it.connState, text),
            )
        }
    }

    fun onClearClicked() {
        syncCoordinator.clearWithRollback()
        _clearEditor.tryEmit(Unit)
        _ui.update { it.copy(editorText = "", charCount = 0, canSendSegment = false) }
    }

    fun onSendSegmentClicked() {
        val text = _ui.value.editorText
        if (text.isEmpty()) {
            _ui.update { it.copy(statusNotice = StatusNotice.EmptyContent) }
            return
        }
        viewModelScope.launch {
            if (syncCoordinator.sendSegment(text).isSuccess) {
                _clearEditor.tryEmit(Unit)
                _ui.update { it.copy(editorText = "", charCount = 0, canSendSegment = false) }
            } else {
                _ui.update { it.copy(statusNotice = StatusNotice.NotConnected) }
            }
        }
    }

    fun onModeChanged(mode: WorkMode) {
        if (_ui.value.workMode == mode) return
        viewModelScope.launch {
            settingsStore.setWorkMode(mode)
            syncCoordinator.setMode(mode)
            _clearEditor.tryEmit(Unit)
            _ui.update {
                it.copy(
                    workMode = mode,
                    editorText = "",
                    charCount = 0,
                    canSendSegment = canSend(mode, it.connState, ""),
                )
            }
        }
    }

    fun onCopyClicked() {
        val text = _ui.value.inboxText
        val ok = text.isNotEmpty() && clipboardWriter.write(text)
        _ui.update {
            it.copy(statusNotice = if (ok) StatusNotice.Copied else StatusNotice.CopyFailed)
        }
    }

    fun onRetryClicked() {
        connectionManager.retryNow()
    }

    fun consumeNotice() {
        _ui.update { it.copy(statusNotice = null) }
    }

    private fun observe() {
        viewModelScope.launch {
            connectionManager.state.collect { state ->
                _ui.update {
                    it.copy(
                        connState = state,
                        canSendSegment = canSend(it.workMode, state, it.editorText),
                    )
                }
            }
        }
        viewModelScope.launch {
            settingsStore.workMode.collect { mode ->
                syncCoordinator.setMode(mode)
                _ui.update {
                    it.copy(
                        workMode = mode,
                        canSendSegment = canSend(mode, it.connState, it.editorText),
                    )
                }
            }
        }
        viewModelScope.launch {
            connectionManager.peerVersion.collect { version ->
                _ui.update { it.copy(peerVersion = version) }
            }
        }
        viewModelScope.launch {
            connectionManager.events.collect(::handleConnectionEvent)
        }
        viewModelScope.launch {
            lifecycleObserver.events.collect { event ->
                if (event == LifecycleEvent.FOREGROUND) onForeground()
            }
        }
    }

    private fun handleConnectionEvent(event: ConnectionEvent) {
        when (event) {
            ConnectionEvent.Opened -> {
                // 连接建立：清空输入框，与电脑端基线同时从空开始
                _clearEditor.tryEmit(Unit)
                _ui.update { it.copy(editorText = "", charCount = 0, canSendSegment = false) }
            }

            is ConnectionEvent.Ack -> Unit

            is ConnectionEvent.Clip -> onClip(event.text)

            ConnectionEvent.BadPin ->
                _ui.update { it.copy(statusNotice = StatusNotice.PinError) }

            ConnectionEvent.BadJson -> Unit
        }
    }

    private fun onClip(text: String) {
        // 空内容保护：不覆盖接收区已有内容
        if (text.isEmpty()) return
        // 后台不展示、不缓存
        if (!lifecycleObserver.isForeground.value) return
        _ui.update { it.copy(inboxText = text, inboxHighlighted = true) }
        viewModelScope.launch {
            delay(HIGHLIGHT_DURATION_MS)
            _ui.update { it.copy(inboxHighlighted = false) }
        }
    }

    private fun onForeground() {
        if (_ui.value.connState == ConnState.Connected) {
            syncCoordinator.flushAlign(_ui.value.editorText)
        }
    }

    private fun canSend(mode: WorkMode, state: ConnState, text: String): Boolean =
        mode == WorkMode.SEGMENT_SEND &&
            state == ConnState.Connected &&
            text.isNotEmpty()

    companion object {
        const val HIGHLIGHT_DURATION_MS = 2000L
    }
}

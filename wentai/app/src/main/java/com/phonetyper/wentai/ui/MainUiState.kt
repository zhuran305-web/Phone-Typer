package com.phonetyper.wentai.ui

import com.phonetyper.wentai.domain.model.ConnState
import com.phonetyper.wentai.domain.model.WorkMode

/** 一次性提示类型，文案在 `strings.xml` 集中映射。 */
enum class StatusNotice {
    NotConnected,
    Copied,
    CopyFailed,
    PinError,
    EmptyContent,
}

/** 主界面唯一状态源。 */
data class MainUiState(
    val connState: ConnState = ConnState.Unpaired,
    val workMode: WorkMode = WorkMode.DEFAULT,
    val editorText: String = "",
    val charCount: Int = 0,
    val inboxText: String = "",
    val inboxHighlighted: Boolean = false,
    val peerVersion: String? = null,
    val appVersion: String = "",
    val canSendSegment: Boolean = false,
    val statusNotice: StatusNotice? = null,
)

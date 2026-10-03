package com.phonetyper.wentai.domain.model

/**
 * 连接状态：五态穷举，作为界面唯一状态源。
 *
 * 文案映射集中定义于 `res/values/strings.xml`，此处不硬编码。
 */
sealed interface ConnState {
    /** 尚未配对，无有效配对信息。 */
    data object Unpaired : ConnState

    /** 正在建立连接。 */
    data object Connecting : ConnState

    /** 已连接，可收发。 */
    data object Connected : ConnState

    /** 已断开，前台退避重连中。 */
    data object Reconnecting : ConnState

    /** PIN 被拒，已停止自动重试，等待用户修正。 */
    data object PinError : ConnState
}

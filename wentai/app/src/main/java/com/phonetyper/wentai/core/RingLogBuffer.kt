package com.phonetyper.wentai.core

/**
 * 运行审计日志：内存环形缓冲，仅会话内存在。
 *
 * 约束（spec 4.3.5）：**不落盘用户输入正文**。调用方只允许传入固定事件常量与
 * 非正文元数据；本类额外做长度截断，避免调用方误传长正文导致正文外泄。
 */
class RingLogBuffer(
    val capacity: Int = DEFAULT_CAPACITY,
    private val maxEventLength: Int = MAX_EVENT_LENGTH,
) {
    private val buffer = ArrayDeque<String>()

    @Synchronized
    fun record(event: String) {
        val sanitized = sanitize(event)
        buffer.addLast(sanitized)
        while (buffer.size > capacity) {
            buffer.removeFirst()
        }
    }

    @Synchronized
    fun recent(): List<String> = buffer.toList()

    @Synchronized
    fun clear() = buffer.clear()

    private fun sanitize(event: String): String {
        val single = event.replace('\n', ' ').replace('\r', ' ')
        return if (single.length > maxEventLength) {
            single.take(maxEventLength) + "…"
        } else {
            single
        }
    }

    companion object {
        const val DEFAULT_CAPACITY = 100
        const val MAX_EVENT_LENGTH = 120

        const val EVENT_CONNECTING = "正在连接电脑端"
        const val EVENT_CONNECTED = "已连接电脑端"
        const val EVENT_DISCONNECTED = "连接已断开，重连中"
        const val EVENT_PIN_REJECTED = "PIN 校验失败，已被拒绝"
        const val EVENT_BACKGROUND = "进入后台，暂停连接调度"
        const val EVENT_FOREGROUND = "回到前台，恢复连接"
    }
}

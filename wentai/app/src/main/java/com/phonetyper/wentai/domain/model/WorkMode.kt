package com.phonetyper.wentai.domain.model

/** 工作模式，默认实时同步。 */
enum class WorkMode {
    LIVE_SYNC,
    SEGMENT_SEND,
    ;

    companion object {
        val DEFAULT = LIVE_SYNC

        fun fromNameOrNull(name: String?): WorkMode? =
            entries.firstOrNull { it.name == name }
    }
}

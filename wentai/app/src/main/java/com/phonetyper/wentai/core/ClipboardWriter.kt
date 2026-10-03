package com.phonetyper.wentai.core

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context

/**
 * 系统剪贴板写入。**仅在用户点击复制时调用**（spec 5.7.5）。
 */
class ClipboardWriter(private val context: Context) {

    fun write(text: String): Boolean {
        if (text.isEmpty()) return false
        // 主路径
        if (setPrimaryClip(text)) return true
        // 回退兼容路径
        return setPrimaryClipCompat(text)
    }

    private fun setPrimaryClip(text: String): Boolean = runCatching {
        val manager = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        manager.setPrimaryClip(ClipData.newPlainText(CLIP_LABEL, text))
    }.isSuccess

    private fun setPrimaryClipCompat(text: String): Boolean = runCatching {
        val manager = context.getSystemService(ClipboardManager::class.java)
        manager.setPrimaryClip(ClipData.newPlainText(CLIP_LABEL, text))
    }.isSuccess

    companion object {
        private const val CLIP_LABEL = "wentai"
    }
}

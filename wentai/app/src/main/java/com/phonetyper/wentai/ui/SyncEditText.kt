package com.phonetyper.wentai.ui

import android.content.Context
import android.util.AttributeSet
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputConnectionWrapper
import androidx.appcompat.widget.AppCompatEditText

/**
 * 包装 `InputConnection` 的输入控件：识别输入法组合态（拼写中）与上屏。
 *
 * `setComposingText` 期间为组合中；`commitText` / `finishComposingText` 后组合结束。
 * 组合态变化会回调 [onComposingChanged]，由界面触发同步判定。
 */
class SyncEditText @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = androidx.appcompat.R.attr.editTextStyle,
) : AppCompatEditText(context, attrs, defStyleAttr) {

    var onComposingChanged: ((Boolean) -> Unit)? = null

    private var composing: Boolean = false

    fun isComposing(): Boolean = composing

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection? {
        val base = super.onCreateInputConnection(outAttrs) ?: return null
        return object : InputConnectionWrapper(base, true) {
            override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
                updateComposing(true)
                return super.setComposingText(text, newCursorPosition)
            }

            override fun finishComposingText(): Boolean {
                updateComposing(false)
                return super.finishComposingText()
            }

            override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
                updateComposing(false)
                return super.commitText(text, newCursorPosition)
            }
        }
    }

    private fun updateComposing(value: Boolean) {
        if (composing == value) return
        composing = value
        onComposingChanged?.invoke(value)
    }
}

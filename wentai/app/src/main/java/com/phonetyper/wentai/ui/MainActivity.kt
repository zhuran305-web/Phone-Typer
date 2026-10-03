package com.phonetyper.wentai.ui

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.widget.addTextChangedListener
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import com.google.android.material.snackbar.Snackbar
import com.phonetyper.wentai.R
import com.phonetyper.wentai.WentaiApp
import com.phonetyper.wentai.databinding.ActivityMainBinding
import com.phonetyper.wentai.domain.model.ConnState
import com.phonetyper.wentai.domain.model.WorkMode
import kotlinx.coroutines.launch

/** 主界面：连接状态、实时同步输入、模式切换、剪贴板接收与复制。 */
class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var viewModel: MainViewModel

    private var notificationRequested = false

    private val notificationPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) {
        // 通知权限仅用于一次性提示，不作后台保活用途
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)

        val container = (application as WentaiApp).container
        viewModel = ViewModelProvider(this, MainViewModelFactory(container))[MainViewModel::class.java]

        setupEditor()
        setupControls()
        observe()
    }

    override fun onStart() {
        super.onStart()
        viewModel.start()
    }

    private fun setupEditor() {
        binding.editor.onComposingChanged = { composing ->
            viewModel.onEditorChanged(
                binding.editor.text?.toString().orEmpty(),
                composing,
            )
        }
        binding.editor.addTextChangedListener { editable ->
            viewModel.onEditorChanged(
                editable?.toString().orEmpty(),
                binding.editor.isComposing(),
            )
        }
    }

    private fun setupControls() {
        binding.btnSend.setOnClickListener { viewModel.onSendSegmentClicked() }
        binding.btnClear.setOnClickListener { viewModel.onClearClicked() }
        binding.btnCopyInbox.setOnClickListener { viewModel.onCopyClicked() }
        binding.btnRetry.setOnClickListener { viewModel.onRetryClicked() }
        binding.btnPair.setOnClickListener {
            startActivity(Intent(this, PairActivity::class.java))
        }
    }

    private fun observe() {
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                launch { viewModel.ui.collect(::render) }
                launch {
                    viewModel.clearEditor.collect {
                        binding.editor.setText("")
                        binding.editor.setSelection(binding.editor.text?.length ?: 0)
                    }
                }
            }
        }
    }

    private fun render(state: MainUiState) {
        binding.tvStatus.setText(statusText(state.connState))
        binding.tvStatus.setTextColor(
            ContextCompat.getColor(this, statusColor(state.connState))
        )

        binding.tvCharCount.text = getString(R.string.char_count, state.charCount)

        binding.tvInbox.text = state.inboxText.ifEmpty { getString(R.string.inbox_empty) }
        binding.btnCopyInbox.isEnabled = state.inboxText.isNotEmpty()

        binding.tvVersionInfo.text = getString(
            R.string.version_info,
            state.appVersion,
            state.peerVersion ?: getString(R.string.version_unknown),
        )

        binding.btnSend.isEnabled = state.canSendSegment
        binding.btnSend.setText(
            if (state.workMode == WorkMode.LIVE_SYNC) {
                R.string.btn_send_live
            } else {
                R.string.btn_send
            }
        )

        binding.inboxCard.strokeColor = ContextCompat.getColor(
            this,
            if (state.inboxHighlighted) R.color.brand_indigo else R.color.border,
        )

        // 模式开关：先解绑再设置，避免渲染触发回调
        binding.switchLive.setOnCheckedChangeListener(null)
        binding.switchLive.isChecked = state.workMode == WorkMode.LIVE_SYNC
        binding.switchLive.setOnCheckedChangeListener { _, checked ->
            viewModel.onModeChanged(
                if (checked) WorkMode.LIVE_SYNC else WorkMode.SEGMENT_SEND
            )
        }

        state.statusNotice?.let { notice ->
            Snackbar.make(binding.root, noticeText(notice), Snackbar.LENGTH_SHORT).show()
            viewModel.consumeNotice()
        }

        if (state.connState == ConnState.Connected) {
            requestNotificationOnce()
        }
    }

    private fun requestNotificationOnce() {
        if (notificationRequested) return
        notificationRequested = true
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            val granted = ContextCompat.checkSelfPermission(
                this,
                Manifest.permission.POST_NOTIFICATIONS,
            ) == PackageManager.PERMISSION_GRANTED
            if (!granted) {
                notificationPermissionLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
            }
        }
    }

    private fun statusText(state: ConnState): Int = when (state) {
        ConnState.Unpaired -> R.string.status_unpaired
        ConnState.Connecting -> R.string.status_connecting
        ConnState.Connected -> R.string.status_connected
        ConnState.Reconnecting -> R.string.status_reconnecting
        ConnState.PinError -> R.string.status_pin_error
    }

    private fun statusColor(state: ConnState): Int = when (state) {
        ConnState.Connected -> R.color.status_ok
        ConnState.Reconnecting, ConnState.PinError -> R.color.status_bad
        else -> R.color.status_wait
    }

    private fun noticeText(notice: StatusNotice): Int = when (notice) {
        StatusNotice.NotConnected -> R.string.notice_not_connected
        StatusNotice.Copied -> R.string.notice_copied
        StatusNotice.CopyFailed -> R.string.notice_copy_failed
        StatusNotice.PinError -> R.string.notice_pin_error
        StatusNotice.EmptyContent -> R.string.notice_empty_content
    }
}

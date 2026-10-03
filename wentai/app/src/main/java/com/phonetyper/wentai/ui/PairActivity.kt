package com.phonetyper.wentai.ui

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import androidx.lifecycle.Lifecycle
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
import com.phonetyper.wentai.R
import com.phonetyper.wentai.WentaiApp
import com.phonetyper.wentai.databinding.ActivityPairBinding
import com.phonetyper.wentai.domain.model.PairingError
import kotlinx.coroutines.launch

/** 配对界面：扫码 / 相册 / 手动三入口。 */
class PairActivity : AppCompatActivity() {

    private lateinit var binding: ActivityPairBinding
    private lateinit var viewModel: PairViewModel

    private val scanLauncher = registerForActivityResult(ScanContract()) { result ->
        val contents = result.contents
        if (contents.isNullOrBlank()) return@registerForActivityResult
        viewModel.onScanned(contents)
    }

    private val galleryLauncher = registerForActivityResult(
        ActivityResultContracts.GetContent()
    ) { uri ->
        if (uri == null) return@registerForActivityResult
        viewModel.onImageSelected(uri)
    }

    private val cameraPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (granted) {
            launchScanner()
        } else {
            showResult(getString(R.string.pair_err_camera_denied))
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityPairBinding.inflate(layoutInflater)
        setContentView(binding.root)

        val container = (application as WentaiApp).container
        viewModel = ViewModelProvider(this, PairViewModelFactory(container))[PairViewModel::class.java]

        binding.btnScan.setOnClickListener { onScanClicked() }
        binding.btnGallery.setOnClickListener { galleryLauncher.launch("image/*") }
        binding.btnManualSave.setOnClickListener { onManualSave() }

        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                viewModel.state.collect { render(it) }
            }
        }
    }

    private fun onScanClicked() {
        val granted = ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA) ==
            PackageManager.PERMISSION_GRANTED
        if (granted) {
            launchScanner()
        } else {
            cameraPermissionLauncher.launch(Manifest.permission.CAMERA)
        }
    }

    private fun launchScanner() {
        val options = ScanOptions().apply {
            setDesiredBarcodeFormats(ScanOptions.QR_CODE)
            setPrompt(getString(R.string.scan_prompt))
            setBeepEnabled(false)
            setOrientationLocked(true)
            setCaptureActivity(PortraitCaptureActivity::class.java)
            setBarcodeImageEnabled(false)
        }
        scanLauncher.launch(options)
    }

    private fun onManualSave() {
        viewModel.onManual(
            host = binding.etHost.text?.toString().orEmpty(),
            httpPort = binding.etHttpPort.text?.toString().orEmpty(),
            wsPort = binding.etWsPort.text?.toString().orEmpty(),
            pin = binding.etPin.text?.toString().orEmpty(),
        )
    }

    private fun render(state: PairUiState) {
        binding.progress.visibility =
            if (state.busy) android.view.View.VISIBLE else android.view.View.GONE
        when (val result = state.result) {
            PairResultUi.Idle -> showResult("")
            PairResultUi.Success -> {
                showResult(getString(R.string.pair_success))
                finish()
            }

            is PairResultUi.Failure -> {
                showResult(getString(errorText(result.error)))
                if (result.error == PairingError.InvalidManual) {
                    binding.etHost.requestFocus()
                }
            }
        }
    }

    private fun showResult(message: String) {
        binding.tvPairResult.text = message
    }

    private fun errorText(error: PairingError): Int = when (error) {
        PairingError.InvalidFormat -> R.string.pair_err_invalid_format
        PairingError.Incomplete -> R.string.pair_err_incomplete
        PairingError.InvalidManual -> R.string.pair_err_manual
        PairingError.ServerUnreachable -> R.string.pair_err_unreachable
        PairingError.InvalidResponse -> R.string.pair_err_response
    }
}

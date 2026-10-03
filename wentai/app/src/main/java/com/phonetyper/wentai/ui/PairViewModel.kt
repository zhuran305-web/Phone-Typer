package com.phonetyper.wentai.ui

import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.phonetyper.wentai.data.QrScanProvider
import com.phonetyper.wentai.domain.PairingUseCase
import com.phonetyper.wentai.domain.model.ManualPairInput
import com.phonetyper.wentai.domain.model.PairingError
import com.phonetyper.wentai.domain.model.PairingResult
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/** 配对界面结果状态。 */
sealed interface PairResultUi {
    data object Idle : PairResultUi
    data object Success : PairResultUi
    data class Failure(val error: PairingError) : PairResultUi
}

data class PairUiState(
    val result: PairResultUi = PairResultUi.Idle,
    val busy: Boolean = false,
)

/** 配对界面 ViewModel：扫码 / 相册 / 手动三入口。 */
class PairViewModel(
    private val pairingUseCase: PairingUseCase,
    private val qrScanProvider: QrScanProvider,
) : ViewModel() {

    private val _state = MutableStateFlow(PairUiState())
    val state: StateFlow<PairUiState> = _state.asStateFlow()

    fun onScanned(raw: String) {
        runPairing { pairingUseCase.applyFromScanned(raw) }
    }

    fun onImageSelected(uri: Uri) {
        viewModelScope.launch {
            _state.value = PairUiState(busy = true)
            val decoded = qrScanProvider.decodeFromImage(uri)
            if (decoded.isNullOrBlank()) {
                _state.value = PairUiState(result = PairResultUi.Failure(PairingError.InvalidFormat))
                return@launch
            }
            applyResult(pairingUseCase.applyFromImage(decoded))
        }
    }

    fun onManual(host: String, httpPort: String, wsPort: String, pin: String) {
        val http = httpPort.trim().toIntOrNull()
        val ws = wsPort.trim().toIntOrNull()
        if (http == null || ws == null) {
            _state.value = PairUiState(result = PairResultUi.Failure(PairingError.InvalidManual))
            return
        }
        runPairing {
            pairingUseCase.applyManual(
                ManualPairInput(
                    host = host.trim(),
                    httpPort = http,
                    wsPort = ws,
                    pin = pin.trim(),
                )
            )
        }
    }

    fun reset() {
        _state.value = PairUiState()
    }

    private fun runPairing(block: suspend () -> PairingResult) {
        viewModelScope.launch {
            _state.value = PairUiState(busy = true)
            applyResult(block())
        }
    }

    private fun applyResult(result: PairingResult) {
        _state.value = when (result) {
            is PairingResult.Success -> PairUiState(result = PairResultUi.Success)
            is PairingResult.Failure -> PairUiState(result = PairResultUi.Failure(result.error))
        }
    }
}

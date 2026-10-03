package com.phonetyper.wentai.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import com.phonetyper.wentai.core.AppContainer

class MainViewModelFactory(private val container: AppContainer) : ViewModelProvider.Factory {

    @Suppress("UNCHECKED_CAST")
    override fun <T : ViewModel> create(modelClass: Class<T>): T {
        if (modelClass.isAssignableFrom(MainViewModel::class.java)) {
            return MainViewModel(
                connectionManager = container.connectionManager,
                syncCoordinator = container.syncCoordinator,
                settingsStore = container.settingsStore,
                clipboardWriter = container.clipboardWriter,
                lifecycleObserver = container.lifecycleObserver,
            ) as T
        }
        throw IllegalArgumentException("未知 ViewModel: ${modelClass.name}")
    }
}

class PairViewModelFactory(private val container: AppContainer) : ViewModelProvider.Factory {

    @Suppress("UNCHECKED_CAST")
    override fun <T : ViewModel> create(modelClass: Class<T>): T {
        if (modelClass.isAssignableFrom(PairViewModel::class.java)) {
            return PairViewModel(
                pairingUseCase = container.pairingUseCase,
                qrScanProvider = container.qrScanProvider,
            ) as T
        }
        throw IllegalArgumentException("未知 ViewModel: ${modelClass.name}")
    }
}

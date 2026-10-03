package com.phonetyper.wentai.core

import android.content.Context
import com.phonetyper.wentai.data.ConfigApi
import com.phonetyper.wentai.data.PairUriParser
import com.phonetyper.wentai.data.PhoneTyperClient
import com.phonetyper.wentai.data.QrScanProvider
import com.phonetyper.wentai.data.Transport
import com.phonetyper.wentai.data.ZxingQrScanProvider
import com.phonetyper.wentai.data.store.PairStore
import com.phonetyper.wentai.data.store.SettingsStore
import com.phonetyper.wentai.data.store.createPairStore
import com.phonetyper.wentai.data.store.createSettingsStore
import com.phonetyper.wentai.domain.ConnectionManager
import com.phonetyper.wentai.domain.PairingUseCase
import com.phonetyper.wentai.domain.SyncCoordinator
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

/**
 * 依赖容器：应用级单例，集中构造各层组件。
 */
class AppContainer(context: Context) {

    private val appContext: Context = context.applicationContext

    val scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    val ringLog = RingLogBuffer()
    val pairStore: PairStore = createPairStore(appContext)
    val settingsStore: SettingsStore = createSettingsStore(appContext)
    val lifecycleObserver = AppLifecycleObserver().also { it.attach() }
    val networkMonitor = NetworkMonitor(appContext)
    val clipboardWriter = ClipboardWriter(appContext)

    val configApi = ConfigApi()
    val pairUriParser = PairUriParser()
    val qrScanProvider: QrScanProvider = ZxingQrScanProvider(appContext)
    val transport: Transport = PhoneTyperClient()

    val connectionManager = ConnectionManager(
        pairStore = pairStore,
        transport = transport,
        lifecycle = lifecycleObserver,
        networkMonitor = networkMonitor,
        configApi = configApi,
        log = ringLog,
        scope = scope,
    )

    val syncCoordinator = SyncCoordinator(
        connectionManager = connectionManager,
        settingsStore = settingsStore,
        scope = scope,
    )

    val pairingUseCase = PairingUseCase(
        parser = pairUriParser,
        configApi = configApi,
        pairStore = pairStore,
    )
}

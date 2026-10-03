package com.phonetyper.wentai.domain

import com.phonetyper.wentai.data.ConfigApi
import com.phonetyper.wentai.data.PairUriParser
import com.phonetyper.wentai.domain.model.ManualPairInput
import com.phonetyper.wentai.domain.model.PairInfo
import com.phonetyper.wentai.domain.model.PairSource
import com.phonetyper.wentai.domain.model.PairingError
import com.phonetyper.wentai.domain.model.PairingResult
import com.phonetyper.wentai.testutil.FakePairStore
import com.phonetyper.wentai.testutil.shortTimeoutClient
import kotlinx.coroutines.test.runTest
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PairingUseCaseTest {

    private val parser = PairUriParser()
    private val store = FakePairStore()

    private fun useCase(clock: () -> Long = { 0L }) =
        PairingUseCase(parser, ConfigApi(shortTimeoutClient()), store, clock)

    @Test
    fun scannedFullUriIsSaved() = runTest {
        val result = useCase { 123L }.applyFromScanned(
            "ptyper://connect?ip=10.0.0.1&http=8766&ws=8767&pin=1"
        )

        assertTrue(result is PairingResult.Success)
        val saved = store.current()!!
        assertEquals("10.0.0.1", saved.host)
        assertEquals(8767, saved.wsPort)
        assertEquals(PairSource.SCAN, saved.source)
        assertEquals(123L, saved.lastUsedAt)
    }

    @Test
    fun missingWsPortIsBackfilledFromConfigApi() = runTest {
        val server = MockWebServer()
        server.start()
        server.enqueue(MockResponse().setBody("""{"ws_port":9999,"version":"1.0.0"}"""))

        val raw = "ptyper://connect?ip=${server.hostName}&http=${server.port}&pin=1"
        val result = useCase().applyFromImage(raw)

        assertTrue(result is PairingResult.Success)
        assertEquals(9999, store.current()!!.wsPort)
        assertEquals(PairSource.GALLERY, store.current()!!.source)
        server.shutdown()
    }

    @Test
    fun unreachableServerKeepsExistingConfig() = runTest {
        val old = PairInfo(host = "old-host", pin = "0")
        store.save(old)

        val result = useCase().applyFromScanned(
            "ptyper://connect?ip=127.0.0.1&http=1&pin=1"
        )

        assertTrue(result is PairingResult.Failure)
        assertEquals("old-host", store.current()!!.host)
    }

    @Test
    fun invalidFormatKeepsExistingConfig() = runTest {
        val old = PairInfo(host = "old-host", pin = "0")
        store.save(old)

        val result = useCase().applyFromScanned("https://not-a-pairing-qr")

        assertEquals(PairingError.InvalidFormat, (result as PairingResult.Failure).error)
        assertEquals("old-host", store.current()!!.host)
    }

    @Test
    fun manualInvalidPortIsRejected() = runTest {
        val result = useCase().applyManual(
            ManualPairInput(host = "1.2.3.4", httpPort = 70000, wsPort = 8767, pin = "1")
        )

        assertEquals(PairingError.InvalidManual, (result as PairingResult.Failure).error)
    }

    @Test
    fun manualValidInputIsSaved() = runTest {
        val result = useCase().applyManual(
            ManualPairInput(host = "1.2.3.4", httpPort = 8766, wsPort = 8767, pin = "12")
        )

        assertTrue(result is PairingResult.Success)
        assertEquals(PairSource.MANUAL, store.current()!!.source)
    }
}

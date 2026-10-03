package com.phonetyper.wentai.domain

import com.phonetyper.wentai.core.RingLogBuffer
import com.phonetyper.wentai.data.ConfigApi
import com.phonetyper.wentai.domain.model.ConnState
import com.phonetyper.wentai.domain.model.PairInfo
import com.phonetyper.wentai.testutil.FakeForegroundState
import com.phonetyper.wentai.testutil.FakeNetworkState
import com.phonetyper.wentai.testutil.FakePairStore
import com.phonetyper.wentai.testutil.FakeTransport
import com.phonetyper.wentai.testutil.shortTimeoutClient
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ConnectionManagerTest {

    private val pairStore = FakePairStore()
    private val transport = FakeTransport()
    private val foreground = FakeForegroundState()
    private val network = FakeNetworkState()

    /** 连接管理器的常驻协程挂在独立 scope 上，避免 runTest 等待其永久结束。 */
    private fun TestScope.newManager(): Pair<ConnectionManager, CoroutineScope> {
        val scope = CoroutineScope(SupervisorJob() + StandardTestDispatcher(testScheduler))
        val manager = ConnectionManager(
            pairStore = pairStore,
            transport = transport,
            lifecycle = foreground,
            networkMonitor = network,
            configApi = ConfigApi(shortTimeoutClient()),
            log = RingLogBuffer(),
            scope = scope,
        )
        return manager to scope
    }

    private suspend fun TestScope.reachConnected(manager: ConnectionManager) {
        pairStore.save(PairInfo(host = "10.0.0.1", pin = "1"))
        testScheduler.advanceUntilIdle()
        transport.emitOpen()
        testScheduler.advanceUntilIdle()
        assertEquals(ConnState.Connected, manager.state.value)
    }

    @Test
    fun pairingTriggersConnectWhenForeground() = runTest {
        val (manager, scope) = newManager()
        pairStore.save(PairInfo(host = "10.0.0.1", pin = "1"))
        testScheduler.advanceUntilIdle()
        assertEquals(ConnState.Connecting, manager.state.value)
        assertEquals(1, transport.connectCalls)
        scope.cancel()
    }

    @Test
    fun openSetsConnectedAndStartsNetworkMonitor() = runTest {
        val (manager, scope) = newManager()
        reachConnected(manager)
        assertTrue(network.started)
        scope.cancel()
    }

    @Test
    fun badPinCircuits() = runTest {
        val (manager, scope) = newManager()
        reachConnected(manager)
        transport.emitMessage("""{"ok":false,"err":"bad-pin"}""")
        testScheduler.advanceUntilIdle()
        assertEquals(ConnState.PinError, manager.state.value)
        scope.cancel()
    }

    @Test
    fun badJsonIsIgnored() = runTest {
        val (manager, scope) = newManager()
        reachConnected(manager)
        transport.emitMessage("""{"ok":false,"err":"bad-json"}""")
        testScheduler.advanceUntilIdle()
        assertEquals(ConnState.Connected, manager.state.value)
        scope.cancel()
    }

    @Test
    fun failureSchedulesBackoffReconnect() = runTest {
        val (manager, scope) = newManager()
        pairStore.save(PairInfo(host = "10.0.0.1", pin = "1"))
        testScheduler.advanceUntilIdle()
        assertEquals(1, transport.connectCalls)

        transport.emitFailure()
        testScheduler.advanceUntilIdle()

        assertEquals(2, transport.connectCalls)
        scope.cancel()
    }

    @Test
    fun backgroundDoesNotReconnectAndStopsNetwork() = runTest {
        val (manager, scope) = newManager()
        reachConnected(manager)

        foreground.setForeground(false)
        testScheduler.advanceUntilIdle()
        transport.emitClosed()
        testScheduler.advanceUntilIdle()

        assertEquals(1, transport.connectCalls)
        assertEquals(ConnState.Connected, manager.state.value)
        assertFalse(network.started)
        scope.cancel()
    }

    @Test
    fun clearingPairingResetsToUnpaired() = runTest {
        val (manager, scope) = newManager()
        reachConnected(manager)
        pairStore.clear()
        testScheduler.advanceUntilIdle()
        assertEquals(ConnState.Unpaired, manager.state.value)
        scope.cancel()
    }
}

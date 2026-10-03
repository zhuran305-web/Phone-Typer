package com.phonetyper.wentai.domain

import com.phonetyper.wentai.core.RingLogBuffer
import com.phonetyper.wentai.data.ConfigApi
import com.phonetyper.wentai.domain.model.PairInfo
import com.phonetyper.wentai.domain.model.WorkMode
import com.phonetyper.wentai.testutil.FakeForegroundState
import com.phonetyper.wentai.testutil.FakeNetworkState
import com.phonetyper.wentai.testutil.FakePairStore
import com.phonetyper.wentai.testutil.FakeSettingsStore
import com.phonetyper.wentai.testutil.FakeTransport
import com.phonetyper.wentai.testutil.shortTimeoutClient
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class SyncCoordinatorTest {

    private val pairStore = FakePairStore()
    private val transport = FakeTransport()
    private val foreground = FakeForegroundState()
    private val network = FakeNetworkState()
    private val settings = FakeSettingsStore()

    private class Harness(
        val manager: ConnectionManager,
        val coordinator: SyncCoordinator,
        val scope: CoroutineScope,
    )

    private fun TestScope.newHarness(): Harness {
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
        val coordinator = SyncCoordinator(manager, settings, scope)
        return Harness(manager, coordinator, scope)
    }

    private suspend fun TestScope.reachConnected(manager: ConnectionManager) {
        pairStore.save(PairInfo(host = "10.0.0.1", pin = "1"))
        testScheduler.advanceUntilIdle()
        transport.emitOpen()
        testScheduler.advanceUntilIdle()
    }

    @Test
    fun debounceMergesRapidInputsIntoOneSync() = runTest {
        val harness = newHarness()
        reachConnected(harness.manager)

        harness.coordinator.onEditorChanged("a", false)
        harness.coordinator.onEditorChanged("ab", false)
        harness.coordinator.onEditorChanged("abc", false)

        advanceTimeBy(100)
        assertTrue(transport.sent.isEmpty())

        advanceTimeBy(100)
        testScheduler.advanceUntilIdle()

        assertEquals(1, transport.sent.size)
        assertEquals("abc", transport.sent.first().text)
        assertEquals("sync", transport.sent.first().type)
        harness.scope.cancel()
    }

    @Test
    fun composingSuppressesSendUntilCommit() = runTest {
        val harness = newHarness()
        reachConnected(harness.manager)

        harness.coordinator.onEditorChanged("nihao", true)
        testScheduler.advanceUntilIdle()
        assertTrue(transport.sent.isEmpty())

        harness.coordinator.onEditorChanged("你好", false)
        testScheduler.advanceUntilIdle()
        assertEquals(1, transport.sent.size)
        assertEquals("你好", transport.sent.first().text)
        harness.scope.cancel()
    }

    @Test
    fun segmentModeDoesNotSyncLive() = runTest {
        val harness = newHarness()
        reachConnected(harness.manager)

        harness.coordinator.setMode(WorkMode.SEGMENT_SEND)
        harness.coordinator.onEditorChanged("x", false)
        testScheduler.advanceUntilIdle()

        assertTrue(transport.sent.isEmpty())
        harness.scope.cancel()
    }

    @Test
    fun clearSendsEmptySyncForRollback() = runTest {
        val harness = newHarness()
        reachConnected(harness.manager)

        harness.coordinator.clearWithRollback()
        testScheduler.advanceUntilIdle()

        assertEquals("", transport.sent.last().text)
        assertEquals("sync", transport.sent.last().type)
        harness.scope.cancel()
    }

    @Test
    fun sendSegmentRejectsEmptyContent() = runTest {
        val harness = newHarness()
        reachConnected(harness.manager)

        assertTrue(harness.coordinator.sendSegment("").isFailure)
        harness.scope.cancel()
    }

    @Test
    fun sendSegmentUsesSendType() = runTest {
        val harness = newHarness()
        reachConnected(harness.manager)

        val result = harness.coordinator.sendSegment("hello")
        testScheduler.advanceUntilIdle()

        assertTrue(result.isSuccess)
        assertEquals("send", transport.sent.last().type)
        assertEquals("hello", transport.sent.last().text)
        harness.scope.cancel()
    }
}

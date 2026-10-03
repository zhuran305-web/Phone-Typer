package com.phonetyper.wentai.data.store

import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import com.phonetyper.wentai.domain.model.WorkMode
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class SettingsStoreTest {

    @get:Rule
    val tmp = TemporaryFolder()

    private fun newStore(): DataStoreSettingsStore {
        val file = File(tmp.root, "settings-${System.nanoTime()}.preferences_pb")
        val dataStore = PreferenceDataStoreFactory.create(
            scope = CoroutineScope(Dispatchers.IO + SupervisorJob()),
            produceFile = { file },
        )
        return DataStoreSettingsStore(dataStore)
    }

    @Test
    fun defaultsToLiveSync() = runTest {
        val store = newStore()
        assertEquals(WorkMode.LIVE_SYNC, store.workMode.first())
    }

    @Test
    fun persistsWorkMode() = runTest {
        val store = newStore()
        store.setWorkMode(WorkMode.SEGMENT_SEND)
        assertEquals(WorkMode.SEGMENT_SEND, store.workMode.first())
    }
}

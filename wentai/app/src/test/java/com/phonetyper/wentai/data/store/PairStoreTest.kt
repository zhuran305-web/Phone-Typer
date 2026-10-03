package com.phonetyper.wentai.data.store

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import com.phonetyper.wentai.domain.model.PairInfo
import com.phonetyper.wentai.domain.model.PairSource
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class PairStoreTest {

    @get:Rule
    val tmp = TemporaryFolder()

    private fun newStore(): Pair<DataStorePairStore, DataStore<Preferences>> {
        val file = File(tmp.root, "pair-${System.nanoTime()}.preferences_pb")
        val dataStore = PreferenceDataStoreFactory.create(
            scope = CoroutineScope(Dispatchers.IO + SupervisorJob()),
            produceFile = { file },
        )
        return DataStorePairStore(dataStore) to dataStore
    }

    @Test
    fun saveReadAndClear() = runTest {
        val (store, _) = newStore()
        val info = PairInfo(
            host = "1.2.3.4",
            httpPort = 8766,
            wsPort = 8767,
            pin = "1",
            source = PairSource.SCAN,
            lastUsedAt = 5L,
        )

        store.save(info)
        assertEquals(info, store.pairing.first())

        store.clear()
        assertNull(store.pairing.first())
    }

    @Test
    fun corruptedJsonIsTolerated() = runTest {
        val (store, dataStore) = newStore()
        dataStore.edit { it[stringPreferencesKey("pair_json")] = "{bad-json" }
        assertNull(store.pairing.first())
    }

    @Test
    fun missingFieldsFallBackToDefaultPorts() = runTest {
        val (store, dataStore) = newStore()
        dataStore.edit {
            it[stringPreferencesKey("pair_json")] = """{"host":"1.2.3.4","pin":"1"}"""
        }

        val info = store.pairing.first()!!
        assertEquals("1.2.3.4", info.host)
        assertEquals(8766, info.httpPort)
        assertEquals(8767, info.wsPort)
    }
}

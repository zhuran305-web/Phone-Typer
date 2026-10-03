package com.phonetyper.wentai.data.store

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import com.phonetyper.wentai.domain.model.WorkMode
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

private val Context.settingsDataStore: DataStore<Preferences> by preferencesDataStore(name = "wentai_settings")

/** 创建应用级设置存储。 */
fun createSettingsStore(context: Context): SettingsStore = DataStoreSettingsStore(context.settingsDataStore)

/** 工作模式持久化，记忆用户上次选择。 */
interface SettingsStore {
    val workMode: Flow<WorkMode>
    suspend fun setWorkMode(mode: WorkMode)
}

class DataStoreSettingsStore(private val dataStore: DataStore<Preferences>) : SettingsStore {

    private val key = stringPreferencesKey("work_mode")

    override val workMode: Flow<WorkMode> = dataStore.data.map { prefs ->
        WorkMode.fromNameOrNull(prefs[key]) ?: WorkMode.DEFAULT
    }

    override suspend fun setWorkMode(mode: WorkMode) {
        dataStore.edit { prefs -> prefs[key] = mode.name }
    }
}

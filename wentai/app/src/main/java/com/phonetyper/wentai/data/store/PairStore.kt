package com.phonetyper.wentai.data.store

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import com.phonetyper.wentai.domain.model.PairInfo
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.serialization.json.Json

private val Context.pairDataStore: DataStore<Preferences> by preferencesDataStore(name = "wentai_pair")

/** 创建应用级配对存储。 */
fun createPairStore(context: Context): PairStore = DataStorePairStore(context.pairDataStore)

/** 配对信息持久化，仅存本机，卸载即清除。 */
interface PairStore {
    val pairing: Flow<PairInfo?>
    suspend fun save(info: PairInfo)
    suspend fun clear()
}

class DataStorePairStore(private val dataStore: DataStore<Preferences>) : PairStore {

    private val key = stringPreferencesKey("pair_json")

    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
        isLenient = true
    }

    override val pairing: Flow<PairInfo?> = dataStore.data.map { prefs -> parse(prefs[key]) }

    override suspend fun save(info: PairInfo) {
        val encoded = json.encodeToString(PairInfo.serializer(), info)
        dataStore.edit { prefs -> prefs[key] = encoded }
    }

    override suspend fun clear() {
        dataStore.edit { prefs -> prefs.remove(key) }
    }

    /** 旧配置/损坏数据容错：无法解析时返回 null，不抛出异常（spec 4.5.4）。 */
    private fun parse(raw: String?): PairInfo? {
        if (raw.isNullOrBlank()) return null
        return runCatching { json.decodeFromString(PairInfo.serializer(), raw) }.getOrNull()
    }
}

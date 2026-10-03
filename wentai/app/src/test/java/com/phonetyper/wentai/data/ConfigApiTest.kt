package com.phonetyper.wentai.data

import com.phonetyper.wentai.domain.model.PairingError
import com.phonetyper.wentai.testutil.shortTimeoutClient
import kotlinx.coroutines.test.runTest
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class ConfigApiTest {

    private lateinit var server: MockWebServer

    @Before
    fun setUp() {
        server = MockWebServer()
        server.start()
    }

    @After
    fun tearDown() {
        runCatching { server.shutdown() }
    }

    @Test
    fun parsesServerConfig() = runTest {
        server.enqueue(MockResponse().setBody("""{"ws_port":8767,"version":"1.0.0"}"""))

        val result = ConfigApi().fetch(server.hostName, server.port)

        assertTrue(result is ConfigFetchResult.Success)
        val config = (result as ConfigFetchResult.Success).config
        assertEquals(8767, config.wsPort)
        assertEquals("1.0.0", config.version)
    }

    @Test
    fun nonJsonResponseIsInvalidResponse() = runTest {
        server.enqueue(MockResponse().setBody("hello"))

        val result = ConfigApi().fetch(server.hostName, server.port)

        assertEquals(PairingError.InvalidResponse, (result as ConfigFetchResult.Failure).error)
    }

    @Test
    fun unreachableServerIsServerUnreachable() = runTest {
        val port = server.port
        val host = server.hostName
        server.shutdown()

        val result = ConfigApi(shortTimeoutClient()).fetch(host, port)

        assertEquals(PairingError.ServerUnreachable, (result as ConfigFetchResult.Failure).error)
    }
}

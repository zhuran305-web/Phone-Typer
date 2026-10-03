package com.phonetyper.wentai.core

import org.junit.Assert.assertEquals
import org.junit.Test

class RingLogBufferTest {

    @Test
    fun rollsOverCapacity() {
        val log = RingLogBuffer(capacity = 3)
        log.record("a")
        log.record("b")
        log.record("c")
        log.record("d")
        assertEquals(listOf("b", "c", "d"), log.recent())
    }

    @Test
    fun truncatesOverlongEventToProtectContent() {
        val log = RingLogBuffer(maxEventLength = 10)
        log.record("x".repeat(50))
        val entry = log.recent().first()
        assertEquals(11, entry.length)
        assertEquals("x".repeat(10) + "…", entry)
    }

    @Test
    fun clearEmptiesBuffer() {
        val log = RingLogBuffer()
        log.record("a")
        log.clear()
        assertEquals(emptyList<String>(), log.recent())
    }
}

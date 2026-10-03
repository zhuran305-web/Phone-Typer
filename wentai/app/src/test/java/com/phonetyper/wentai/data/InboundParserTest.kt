package com.phonetyper.wentai.data

import com.phonetyper.wentai.domain.model.ErrorCode
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class InboundParserTest {

    @Test
    fun parsesSyncAck() {
        val result = InboundParser.parse("""{"ok":true,"bs":2,"typed":3,"s":1}""")
        assertTrue(result is Inbound.Ack)
        val ack = (result as Inbound.Ack).ack
        assertEquals(2, ack.bs)
        assertEquals(3, ack.typed)
        assertEquals(1, ack.s)
    }

    @Test
    fun parsesSendAck() {
        val result = InboundParser.parse("""{"ok":true}""")
        assertTrue(result is Inbound.Ack)
    }

    @Test
    fun parsesBadPin() {
        val result = InboundParser.parse("""{"ok":false,"err":"bad-pin"}""")
        assertEquals(ErrorCode.BAD_PIN, (result as Inbound.Error).code)
    }

    @Test
    fun parsesBadJson() {
        val result = InboundParser.parse("""{"ok":false,"err":"bad-json"}""")
        assertEquals(ErrorCode.BAD_JSON, (result as Inbound.Error).code)
    }

    @Test
    fun parsesClip() {
        val result = InboundParser.parse("""{"type":"clip","text":"hello"}""")
        assertEquals("hello", (result as Inbound.Clip).text)
    }

    @Test
    fun unknownStructureIsIgnored() {
        assertEquals(Inbound.Unknown, InboundParser.parse("""{"foo":1}"""))
    }

    @Test
    fun invalidJsonIsIgnored() {
        assertEquals(Inbound.Unknown, InboundParser.parse("not-json"))
    }
}

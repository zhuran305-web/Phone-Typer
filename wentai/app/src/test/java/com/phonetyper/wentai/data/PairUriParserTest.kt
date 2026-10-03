package com.phonetyper.wentai.data

import com.phonetyper.wentai.domain.model.PairingError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PairUriParserTest {

    private val parser = PairUriParser()

    @Test
    fun parsesFullUri() {
        val result = parser.parse(
            "ptyper://connect?ip=192.168.1.10&http=8766&ws=8767&pin=1234"
        )
        assertTrue(result is PairParseResult.Ok)
        result as PairParseResult.Ok
        assertEquals("192.168.1.10", result.host)
        assertEquals(8766, result.httpPort)
        assertEquals(8767, result.wsPort)
        assertEquals("1234", result.pin)
    }

    @Test
    fun missingWsPortIsPendingNotFailure() {
        val result = parser.parse("ptyper://connect?ip=10.0.0.2&http=8766&pin=0000")
        assertTrue(result is PairParseResult.Ok)
        assertNull((result as PairParseResult.Ok).wsPort)
    }

    @Test
    fun httpPortDefaultsWhenAbsent() {
        val result = parser.parse("ptyper://connect?ip=10.0.0.2&ws=8767&pin=1")
        assertEquals(8766, (result as PairParseResult.Ok).httpPort)
    }

    @Test
    fun missingIpIsIncomplete() {
        val result = parser.parse("ptyper://connect?http=8766&pin=1")
        assertEquals(PairingError.Incomplete, (result as PairParseResult.Error).error)
    }

    @Test
    fun missingPinIsIncomplete() {
        val result = parser.parse("ptyper://connect?ip=1.2.3.4&http=8766")
        assertEquals(PairingError.Incomplete, (result as PairParseResult.Error).error)
    }

    @Test
    fun wrongSchemeIsInvalidFormat() {
        val result = parser.parse("https://connect?ip=1.2.3.4&pin=1")
        assertEquals(PairingError.InvalidFormat, (result as PairParseResult.Error).error)
    }

    @Test
    fun portOutOfRangeIsInvalidFormat() {
        val result = parser.parse("ptyper://connect?ip=1.2.3.4&http=70000&pin=1")
        assertEquals(PairingError.InvalidFormat, (result as PairParseResult.Error).error)
    }

    @Test
    fun urlEncodedParamsAreDecoded() {
        val result = parser.parse("ptyper://connect?ip=192.168.1.1&pin=a%2Bb")
        assertEquals("a+b", (result as PairParseResult.Ok).pin)
    }

    @Test
    fun blankInputIsInvalidFormat() {
        assertEquals(PairingError.InvalidFormat, (parser.parse("  ") as PairParseResult.Error).error)
    }
}

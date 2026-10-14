package app.sela.playback

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class QueuePolicyTest {
    @Test fun queueBoundariesPreserveEntryOrder() {
        assertNull(queueIndex(0, 1, 0, "queue"))
        assertEquals(0, queueIndex(-1, 1, 3, "off"))
        assertEquals(2, queueIndex(1, 1, 3, "off"))
        assertNull(queueIndex(2, 1, 3, "off"))
        assertNull(queueIndex(0, -1, 3, "off"))
        assertEquals(0, queueIndex(2, 1, 3, "queue"))
        assertEquals(2, queueIndex(0, -1, 3, "queue"))
        assertEquals(0, queueIndex(0, 1, 1, "queue"))
    }
}

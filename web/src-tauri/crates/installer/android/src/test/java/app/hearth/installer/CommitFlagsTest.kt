package app.hearth.installer

import android.app.PendingIntent
import org.junit.Assert.assertTrue
import org.junit.Test

class CommitFlagsTest {
    @Test
    fun commitReceiverStaysMutable() {
        val flags = commitPendingIntentFlags()
        assertTrue(flags and PendingIntent.FLAG_MUTABLE != 0)
        assertTrue(flags and PendingIntent.FLAG_IMMUTABLE == 0)
        assertTrue(flags and PendingIntent.FLAG_UPDATE_CURRENT != 0)
    }
}

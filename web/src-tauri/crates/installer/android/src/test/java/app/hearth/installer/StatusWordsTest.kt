package app.hearth.installer

import android.content.pm.PackageInstaller
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class StatusWordsTest {
    @Test
    fun userActionAsksForInstallTap() {
        assertEquals("Tap Install to finish.", statusWords(PackageInstaller.STATUS_PENDING_USER_ACTION))
    }

    @Test
    fun successAndStopStayShort() {
        assertEquals("Install finished.", statusWords(PackageInstaller.STATUS_SUCCESS))
        assertEquals("Install stopped.", statusWords(PackageInstaller.STATUS_FAILURE_ABORTED))
    }

    @Test
    fun failuresNameTheNextStep() {
        assertTrue(statusWords(PackageInstaller.STATUS_FAILURE_BLOCKED).contains("Check the file."))
        assertTrue(statusWords(PackageInstaller.STATUS_FAILURE_STORAGE).contains("storage is full."))
        assertTrue(statusWords(-99).contains("Retry the update."))
    }
}

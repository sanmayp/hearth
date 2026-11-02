package app.hearth.installer

import android.app.Activity
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.os.Build
import android.provider.Settings
import android.util.Log
import android.widget.Toast
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin
import java.io.File
import java.io.FileInputStream

@InvokeArg
class InstallArgs {
    lateinit var path: String
}

/// Flags for the commit status receiver. PackageInstaller fills the
/// status extras at commit time, so the intent must stay mutable.
/// API 31+ rejects immutable receivers with a commit() error.
fun commitPendingIntentFlags(): Int =
    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE

/// Plain words for each PackageInstaller status. Short lines only.
fun statusWords(status: Int): String = when (status) {
    PackageInstaller.STATUS_PENDING_USER_ACTION -> "Tap Install to finish."
    PackageInstaller.STATUS_SUCCESS -> "Install finished."
    PackageInstaller.STATUS_FAILURE_ABORTED -> "Install stopped."
    PackageInstaller.STATUS_FAILURE_BLOCKED -> "Install is blocked. Check the file."
    PackageInstaller.STATUS_FAILURE_CONFLICT -> "Install conflicts. Remove the old app first."
    PackageInstaller.STATUS_FAILURE_INCOMPATIBLE -> "APK does not fit this phone."
    PackageInstaller.STATUS_FAILURE_INVALID -> "APK file is bad. Download it again."
    PackageInstaller.STATUS_FAILURE_STORAGE -> "Phone storage is full."
    else -> "Install failed. Retry the update."
}

/// Commit status receiver. It logs each status, shows plain words,
/// and starts the system installer screen on user action.
class InstallStatusReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, -99)
        val detail = intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE)
        Log.i("HearthInstaller", "status=$status detail=$detail")
        Toast.makeText(context, statusWords(status), Toast.LENGTH_LONG).show()
        if (status == PackageInstaller.STATUS_PENDING_USER_ACTION) {
            val launch = intent.getParcelableExtra<Intent>(Intent.EXTRA_INTENT)
            if (launch == null) {
                Log.w("HearthInstaller", "user action asked, no launch intent")
                return
            }
            launch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            try {
                context.startActivity(launch)
            } catch (ex: Exception) {
                Log.w("HearthInstaller", "launch failed: ${ex.message}")
            }
        }
    }
}

/// Installs a staged APK through a PackageInstaller session.
/// Android shows the system confirm dialog. Unknown sources must be
/// allowed once; the user is sent to Settings when they are not.
@TauriPlugin
class InstallerPlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun install(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(InstallArgs::class.java)
            val file = File(args.path)
            if (!file.isFile) {
                invoke.reject("staged file is gone")
                return
            }
            val pm = activity.packageManager
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !pm.canRequestPackageInstalls()) {
                val settings = Intent(
                    Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                    android.net.Uri.parse("package:${activity.packageName}")
                )
                settings.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                activity.startActivity(settings)
                invoke.reject("Allow installs for Hearth. Tap Allow, then tap Update again.")
                return
            }
            val installer = pm.packageInstaller
            val params = PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL)
            params.setAppPackageName(activity.packageName)
            val sessionId = installer.createSession(params)
            val session = installer.openSession(sessionId)
            try {
                FileInputStream(file).use { input ->
                    session.openWrite("hearth", 0, file.length()).use { output ->
                        input.copyTo(output)
                        session.fsync(output)
                    }
                }
                val status = Intent(activity, InstallStatusReceiver::class.java)
                    .setAction("app.hearth.installer.STATUS")
                val done = PendingIntent.getBroadcast(
                    activity, sessionId, status, commitPendingIntentFlags()
                )
                session.commit(done.intentSender)
                invoke.resolve()
            } finally {
                session.close()
            }
        } catch (ex: Exception) {
            invoke.reject(ex.message)
        }
    }
}

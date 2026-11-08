package io.github.wobi848.modbusio

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.IBinder
import android.os.PowerManager
import android.util.Log
import java.io.File
import java.io.IOException
import java.net.HttpURLConnection
import java.net.InetAddress
import java.net.ServerSocket
import java.net.URL
import kotlin.concurrent.thread

/**
 * Keeps modbusio running, also with the screen off (recordings): a foreground service with a
 * notification that starts the program (libmodbusio.so) and starts it again after it ended.
 */
class ModbusioService : Service() {
    private lateinit var wakeLock: PowerManager.WakeLock
    @Volatile private var process: Process? = null
    @Volatile private var stopping = false

    override fun onCreate() {
        super.onCreate()
        startForeground(1, notification(), ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE)
        wakeLock = (getSystemService(POWER_SERVICE) as PowerManager)
            .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "modbusio:run")
            .apply { acquire() }
        webPort = choosePort()
        thread(name = "modbusio") { supervise() }
    }

    /**
     * The port kept from last time, at first [PREFERRED_PORT]; another free one only when another
     * app holds it. The page keeps its settings per port, so the port changes as seldom as possible.
     */
    private fun choosePort(): Int {
        val kept = getSharedPreferences(PREFERENCES, MODE_PRIVATE)
        val port = kept.getInt(PORT_KEY, PREFERRED_PORT)
        if (isFree(port)) return port
        val other = ServerSocket(0, 1, InetAddress.getLoopbackAddress()).use { it.localPort }
        Log.i(TAG, "port $port is taken, using $other")
        kept.edit().putInt(PORT_KEY, other).apply()
        return other
    }

    private fun isFree(port: Int): Boolean = try {
        ServerSocket(port, 1, InetAddress.getLoopbackAddress()).close()
        true
    } catch (e: IOException) {
        false
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) stopSelf()
        return START_STICKY
    }

    override fun onDestroy() {
        stopping = true
        process?.destroy()
        wakeLock.release()
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun supervise() {
        while (!stopping) {
            val started = System.currentTimeMillis()
            val running = start()
            process = running
            val code = running.waitFor()
            process = null
            Log.i(TAG, "modbusio ended with $code")
            // crashed right away again: do not spin
            if (System.currentTimeMillis() - started < 5000) Thread.sleep(5000)
        }
    }

    private fun start(): Process {
        val log = File(filesDir, "modbusio.log")
        if (log.length() > LOG_MAX) log.renameTo(File(filesDir, "modbusio.log.1"))
        return ProcessBuilder(
            File(applicationInfo.nativeLibraryDir, "libmodbusio.so").path,
            "--no-browser",
            "--web-port", webPort.toString(),
            "--data-dir", File(filesDir, "data").path,
        ).apply {
            // an app may not write to Android's /data/local/tmp
            environment()["TMPDIR"] = cacheDir.path
            redirectErrorStream(true)
            redirectOutput(ProcessBuilder.Redirect.appendTo(log))
        }.start()
    }

    private fun notification(): Notification {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL, getString(R.string.channel), NotificationManager.IMPORTANCE_LOW)
        )
        val open = PendingIntent.getActivity(
            this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE
        )
        val stop = PendingIntent.getService(
            this, 1, Intent(this, ModbusioService::class.java).setAction(ACTION_STOP), PendingIntent.FLAG_IMMUTABLE
        )
        return Notification.Builder(this, CHANNEL)
            .setSmallIcon(R.drawable.ic_launcher)
            .setContentTitle(getString(R.string.running))
            .setContentText(getString(R.string.running_text))
            .setContentIntent(open)
            .addAction(Notification.Action.Builder(null, getString(R.string.stop), stop).build())
            .setOngoing(true)
            .build()
    }

    companion object {
        private const val PREFERRED_PORT = 8082
        private const val PREFERENCES = "modbusio"
        private const val PORT_KEY = "webPort"
        const val ACTION_STOP = "io.github.wobi848.modbusio.STOP"
        private const val CHANNEL = "run"
        private const val TAG = "modbusio"
        private const val LOG_MAX = 1024 * 1024L

        /** Where the page is served; chosen when the service starts. */
        @Volatile var webPort = PREFERRED_PORT
            private set

        /** Does modbusio answer yet, and not some other program on the port? */
        fun reachable(): Boolean = try {
            val connection = URL("http://127.0.0.1:$webPort/api/info").openConnection() as HttpURLConnection
            connection.connectTimeout = 500
            connection.readTimeout = 1000
            connection.responseCode == 200 && connection.inputStream.use { it.reader().readText() }.contains("\"program\":\"modbusio\"")
        } catch (e: IOException) {
            false
        }
    }
}

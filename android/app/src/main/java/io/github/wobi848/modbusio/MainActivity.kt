package io.github.wobi848.modbusio

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.app.AlertDialog
import android.content.ContentValues
import android.content.Intent
import android.content.res.Configuration
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.provider.MediaStore
import android.view.WindowInsets
import android.window.OnBackInvokedDispatcher
import android.view.WindowInsetsController
import android.webkit.URLUtil
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout
import android.widget.Toast
import org.json.JSONObject
import org.json.JSONTokener
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import kotlin.concurrent.thread

/** The modbusio web UI in the whole window; modbusio itself runs in ModbusioService. */
class MainActivity : Activity() {
    private lateinit var web: WebView
    private val home get() = "http://127.0.0.1:${ModbusioService.webPort}/"

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 0)
        startForegroundService(Intent(this, ModbusioService::class.java))

        web = WebView(this)
        web.settings.javaScriptEnabled = true
        web.settings.domStorageEnabled = true
        web.webViewClient = object : WebViewClient() {
            // other links in the browser, modbusio itself here
            override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                if (request.url.toString().startsWith(home)) return false
                startActivity(Intent(Intent.ACTION_VIEW, request.url))
                return true
            }
        }
        web.setDownloadListener { url, _, disposition, mime, _ ->
            download(url, URLUtil.guessFileName(url, disposition, mime), mime)
        }
        web.loadData(getString(R.string.starting), "text/plain; charset=utf-8", null)
        setContentView(clearOfSystemBars(web))
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            onBackInvokedDispatcher.registerOnBackInvokedCallback(OnBackInvokedDispatcher.PRIORITY_DEFAULT) { back() }
        }
        waitForModbusio()
    }

    /**
     * Since Android 15 the app draws under the status bar, the camera cutout and the navigation
     * bar: the page keeps clear of them, and of the keyboard, so nothing hides behind them. The
     * strips take the colour of the interface (web/css/tokens.css, --surface).
     */
    private fun clearOfSystemBars(content: WebView): FrameLayout {
        val night = resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK ==
            Configuration.UI_MODE_NIGHT_YES
        val frame = FrameLayout(this)
        frame.setBackgroundColor(Color.parseColor(if (night) "#141c2b" else "#ffffff"))
        frame.addView(content)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            frame.setOnApplyWindowInsetsListener { view, insets ->
                val bars = insets.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout())
                val keyboard = insets.getInsets(WindowInsets.Type.ime())
                view.setPadding(bars.left, bars.top, bars.right, maxOf(bars.bottom, keyboard.bottom))
                WindowInsets.CONSUMED
            }
            val dark = WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS or
                WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS
            frame.post { window.insetsController?.setSystemBarsAppearance(if (night) 0 else dark, dark) }
        }
        return frame
    }

    private fun waitForModbusio() = thread {
        repeat(60) {
            if (ModbusioService.reachable()) {
                runOnUiThread { web.loadUrl(home) }
                return@thread
            }
            Thread.sleep(500)
        }
        runOnUiThread { web.loadData(getString(R.string.failed), "text/plain; charset=utf-8", null) }
    }

    /** CSV exports go to Downloads. */
    private fun download(url: String, name: String, mime: String?) = thread {
        val message = try {
            val connection = URL(url).openConnection() as HttpURLConnection
            val values = ContentValues().apply {
                put(MediaStore.Downloads.DISPLAY_NAME, name)
                put(MediaStore.Downloads.MIME_TYPE, mime ?: "application/octet-stream")
            }
            val target = contentResolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values)
                ?: throw IOException(name)
            connection.inputStream.use { input ->
                contentResolver.openOutputStream(target)!!.use { input.copyTo(it) }
            }
            getString(R.string.saved, name)
        } catch (e: IOException) {
            getString(R.string.save_failed, e.message)
        }
        runOnUiThread { Toast.makeText(this, message, Toast.LENGTH_LONG).show() }
    }

    @Deprecated("Activity without AndroidX")
    override fun onBackPressed() = back()

    /** The back key steps back in the page (web/app.js, modbusioBack); with no step left it asks. */
    private fun back() {
        web.evaluateJavascript("window.modbusioBack ? window.modbusioBack() : null") { result ->
            val answer = runCatching { JSONObject(JSONTokener(result).nextValue() as String) }.getOrNull()
            if (answer?.getBoolean("stepped") != true) askToLeave(answer?.getInt("held") ?: 0)
        }
    }

    private fun askToLeave(held: Int) {
        val text = getString(R.string.leave_text) +
            if (held > 0) "\n\n" + resources.getQuantityString(R.plurals.leave_held, held, held) else ""
        AlertDialog.Builder(this)
            .setTitle(R.string.leave_title)
            .setMessage(text)
            .setPositiveButton(R.string.leave) { _, _ -> moveTaskToBack(true) }
            .setNeutralButton(R.string.stop) { _, _ ->
                startService(Intent(this, ModbusioService::class.java).setAction(ModbusioService.ACTION_STOP))
                finishAndRemoveTask()
            }
            .setNegativeButton(R.string.stay, null)
            .show()
    }
}

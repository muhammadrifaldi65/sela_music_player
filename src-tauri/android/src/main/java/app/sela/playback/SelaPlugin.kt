package app.sela.playback

import android.app.Activity
import android.content.Intent
import android.os.Handler
import android.os.Looper
import android.provider.OpenableColumns
import androidx.activity.result.ActivityResult
import androidx.media3.common.util.UnstableApi
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import org.json.JSONObject
import java.util.concurrent.Executors

@UnstableApi
@TauriPlugin
class SelaPlugin(private val activity: Activity) : Plugin(activity) {
    companion object {
        @JvmStatic external fun nativeStore(request: String): String
    }
    private val main = Handler(Looper.getMainLooper())
    private val searches = Executors.newSingleThreadExecutor()
    private fun withService(invoke: Invoke, action: (PlaybackService) -> Unit) {
        main.post {
            val actionRunner: (PlaybackService) -> Unit = { service ->
                try { action(service) } catch (failure: Exception) { invoke.reject(failure.message ?: "Perintah gagal") }
            }
            PlaybackService.instance?.let(actionRunner) ?: run {
                PlaybackService.waiting.add(actionRunner)
                try { activity.startService(Intent(activity, PlaybackService::class.java)) }
                catch (failure: Exception) { PlaybackService.waiting.remove(actionRunner); invoke.reject(failure.message) }
            }
        }
    }
    @Command fun search(invoke: Invoke) {
        val query = invoke.getArgs().optString("query")
        searches.execute {
            try { invoke.resolve(JSObject().put("tracks", Source.search(query)) as JSObject) }
            catch (failure: Exception) { invoke.reject(failure.message ?: "Pencarian gagal") }
        }
    }
    @Command fun play(invoke: Invoke) = withService(invoke) { service ->
        val args = invoke.getArgs()
        service.play(args.getJSONObject("track"), if (args.isNull("entryId")) null else args.getLong("entryId")) {
            if (it == null) invoke.resolve(JSObject()) else invoke.reject(it)
        }
    }
    @Command fun syncQueue(invoke: Invoke) = withService(invoke) {
        it.sync(PlaybackService.store(JSONObject().put("action", "snapshot")) as JSONObject); invoke.resolve(JSObject())
    }
    @Command fun playbackState(invoke: Invoke) = withService(invoke) { invoke.resolve(JSObject(it.state().toString())) }
    @Command fun setPaused(invoke: Invoke) = withService(invoke) {
        if (invoke.getArgs().getBoolean("paused")) it.player.pause() else it.player.play()
        invoke.resolve(JSObject())
    }
    @Command fun seek(invoke: Invoke) = withService(invoke) {
        val seconds = invoke.getArgs().getDouble("seconds")
        require(seconds.isFinite() && seconds >= 0 && seconds <= Long.MAX_VALUE / 1000.0) { "Posisi tidak valid" }
        it.player.seekTo((seconds * 1000).toLong()); invoke.resolve(JSObject())
    }
    @Command fun setVolume(invoke: Invoke) = withService(invoke) {
        val volume = invoke.getArgs().getDouble("volume")
        require(volume.isFinite() && volume in 0.0..100.0) { "Volume tidak valid" }
        it.player.volume = (volume / 100).toFloat(); invoke.resolve(JSObject())
    }
    @Command fun next(invoke: Invoke) = withService(invoke) { it.advance(1, false); invoke.resolve(JSObject()) }
    @Command fun previous(invoke: Invoke) = withService(invoke) { it.advance(-1, false); invoke.resolve(JSObject()) }
    @Command fun openLocal(invoke: Invoke) {
        main.post {
            try {
                startActivityForResult(invoke, Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                    type = "audio/*"
                    addCategory(Intent.CATEGORY_OPENABLE)
                    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
                }, "localResult")
            } catch (failure: Exception) { invoke.reject(failure.message) }
        }
    }
    @ActivityCallback fun localResult(invoke: Invoke, result: ActivityResult) {
        if (result.resultCode != Activity.RESULT_OK) { invoke.resolve(JSObject()); return }
        try {
            val uri = result.data?.data ?: error("Berkas tidak dipilih")
            require(uri.scheme == "content") { "URI berkas tidak valid" }
            activity.contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
            var title = "Audio lokal"
            activity.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
                if (it.moveToFirst()) title = it.getString(0) ?: title
            }
            val track = JSONObject().put("id", uri.toString()).put("title", title).put("channel", JSONObject.NULL)
            withService(invoke) { service -> service.play(track, null) {
                if (it == null) invoke.resolve(JSObject(track.toString())) else invoke.reject(it)
            } }
        } catch (failure: Exception) { invoke.reject(failure.message ?: "Berkas tidak dapat dibuka") }
    }
}

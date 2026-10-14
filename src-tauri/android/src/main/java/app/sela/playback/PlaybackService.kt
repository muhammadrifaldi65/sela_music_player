package app.sela.playback

import android.app.PendingIntent
import android.content.Intent
import android.net.Uri
import android.os.Handler
import android.os.Looper
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.ForwardingPlayer
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import org.json.JSONArray
import org.json.JSONObject
import java.util.UUID
import java.util.concurrent.Executors

@UnstableApi
class PlaybackService : MediaSessionService() {
    companion object {
        var instance: PlaybackService? = null
        val waiting = mutableListOf<(PlaybackService) -> Unit>()
        fun store(request: JSONObject): Any? {
            val response = try { SelaPlugin.nativeStore(request.toString()) }
                catch (failure: UnsatisfiedLinkError) { throw IllegalStateException("Buka aplikasi untuk mengaktifkan penyimpanan", failure) }
            val result = JSONObject(response)
            check(result.getBoolean("ok")) { result.optString("error", "Penyimpanan gagal") }
            return result.opt("value")
        }
    }
    lateinit var player: ExoPlayer
    private var session: MediaSession? = null
    private val main = Handler(Looper.getMainLooper())
    private val extraction = Executors.newSingleThreadExecutor()
    private var generation = 0
    private var entries = JSONArray()
    var repeat = "off"
    var autoplay = false
    private var current: JSONObject? = null
    private var entryId: Long? = null
    private var related = emptyList<JSONObject>()
    private var error: String? = null
    private var resolving = false
    private var sessionId = ""
    private var recorded = false

    override fun onCreate() {
        super.onCreate()
        player = ExoPlayer.Builder(this).build().apply {
            setAudioAttributes(AudioAttributes.Builder().setUsage(C.USAGE_MEDIA)
                .setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(), true)
            setHandleAudioBecomingNoisy(true)
            setWakeMode(C.WAKE_MODE_NETWORK)
            trackSelectionParameters = trackSelectionParameters.buildUpon().setTrackTypeDisabled(C.TRACK_TYPE_VIDEO, true).build()
            addListener(object : Player.Listener {
                override fun onPlaybackStateChanged(state: Int) {
                    if (state == Player.STATE_ENDED) guarded { advance(1, true) }
                }
                override fun onIsPlayingChanged(isPlaying: Boolean) {
                    if (isPlaying && !recorded && entryId != null) guarded {
                        store(JSONObject().put("action", "listen").put("entry_id", entryId).put("session", sessionId))
                        recorded = true
                    }
                }
                override fun onPlayerError(failure: PlaybackException) {
                    error = "Pemutaran gagal (${failure.errorCodeName})"
                    resolving = false
                }
            })
        }
        val controls = object : ForwardingPlayer(player) {
            override fun getAvailableCommands(): Player.Commands = super.getAvailableCommands().buildUpon()
                .add(Player.COMMAND_SEEK_TO_NEXT).add(Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM)
                .add(Player.COMMAND_SEEK_TO_PREVIOUS).add(Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM).build()
            override fun isCommandAvailable(command: Int) = availableCommands.contains(command)
            override fun seekToNext() { guarded { advance(1, false) } }
            override fun seekToNextMediaItem() { seekToNext() }
            override fun seekToPrevious() { guarded { advance(-1, false) } }
            override fun seekToPreviousMediaItem() { seekToPrevious() }
        }
        val builder = MediaSession.Builder(this, controls)
        packageManager.getLaunchIntentForPackage(packageName)?.let { launch ->
            builder.setSessionActivity(PendingIntent.getActivity(this, 0, launch,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE))
        }
        session = builder.build()
        guarded { sync(store(JSONObject().put("action", "snapshot")) as JSONObject) }
        instance = this
        waiting.toList().also { waiting.clear() }.forEach { it(this) }
    }
    private fun guarded(action: () -> Unit) {
        try { action() } catch (failure: Exception) { error = failure.message ?: "Pemutaran gagal" }
    }
    fun sync(snapshot: JSONObject) {
        val mode = snapshot.getString("repeat")
        require(mode in listOf("off", "once", "song", "queue"))
        entries = snapshot.getJSONArray("entries")
        repeat = mode
        autoplay = snapshot.getBoolean("autoplay")
    }
    fun play(track: JSONObject, id: Long?, complete: (String?) -> Unit = {}) {
        val trackId = track.getString("id")
        require(trackId.matches(Regex("[A-Za-z0-9_-]{11}")) || Uri.parse(trackId).scheme == "content")
        require(track.getString("title").isNotBlank())
        val request = ++generation
        player.stop()
        error = null
        resolving = true
        current = track
        entryId = id
        recorded = false
        sessionId = UUID.randomUUID().toString()
        extraction.execute {
            val result = runCatching { Source.resolve(track) }
            main.post {
                if (request != generation) { complete("Permintaan pemutaran diganti"); return@post }
                result.fold({ source ->
                    try {
                        if (id != null) store(JSONObject().put("action", "select").put("entry_id", id))
                        related = source.related
                        val metadata = MediaMetadata.Builder().setTitle(track.getString("title"))
                            .setArtist(track.optString("channel", ""))
                        source.artwork?.let { metadata.setArtworkUri(Uri.parse(it)) }
                        player.setMediaItem(MediaItem.Builder().setMediaId(trackId).setUri(source.uri)
                            .setMimeType(source.mime).setMediaMetadata(metadata.build()).build())
                        player.prepare()
                        player.play()
                        resolving = false
                        complete(null)
                    } catch (failure: Exception) {
                        error = "Pemutaran gagal (${failure.javaClass.simpleName})"; resolving = false; complete(error)
                    }
                }, { failure ->
                    error = failure.message ?: "Ekstraksi audio gagal"
                    resolving = false
                    complete(error)
                })
            }
        }
    }
    fun advance(direction: Int, automatic: Boolean) {
        val index = (0 until entries.length()).firstOrNull { entries.getJSONObject(it).getLong("entry_id") == entryId } ?: -1
        if (automatic && index < 0) return
        if (automatic && repeat in listOf("once", "song")) {
            if (repeat == "once") {
                store(JSONObject().put("action", "options").put("repeat", "off").put("autoplay", autoplay))
                repeat = "off"
            }
            current?.let { play(it, entryId) }
            return
        }
        if (automatic && !autoplay && repeat != "queue") return
        if (direction < 0 && player.currentPosition > 3000) { player.seekTo(0); return }
        val next = queueIndex(index, direction, entries.length(), repeat)
        if (next != null) {
            val row = entries.getJSONObject(next)
            play(row.getJSONObject("track"), row.getLong("entry_id"))
        } else if (direction > 0 && autoplay) {
            val queued = (0 until entries.length()).map { entries.getJSONObject(it).getJSONObject("track").getString("id") }.toSet()
            val candidate = related.firstOrNull { it.getString("id") != current?.optString("id") && it.getString("id") !in queued }
            if (candidate != null) {
                val added = store(JSONObject().put("action", "append").put("track", candidate)) as JSONObject
                sync(store(JSONObject().put("action", "snapshot")) as JSONObject)
                play(candidate, added.getLong("entry_id"))
            }
        }
    }
    fun state(): JSONObject = JSONObject().put("position", player.currentPosition / 1000.0)
        .put("duration", if (player.duration == C.TIME_UNSET) 0.0 else player.duration / 1000.0)
        .put("paused", !player.playWhenReady).put("ended", player.playbackState == Player.STATE_ENDED)
        .put("loading", resolving || player.playbackState == Player.STATE_BUFFERING)
        .put("volume", player.volume * 100.0).put("track", current ?: JSONObject.NULL)
        .put("entryId", entryId ?: JSONObject.NULL).put("error", error ?: JSONObject.NULL)
        .put("repeat", repeat).put("autoplay", autoplay)
    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? = session
    override fun onTaskRemoved(rootIntent: Intent?) { if (!player.playWhenReady) stopSelf() }
    override fun onDestroy() {
        generation++
        instance = null
        extraction.shutdownNow()
        session?.release()
        player.release()
        super.onDestroy()
    }
}

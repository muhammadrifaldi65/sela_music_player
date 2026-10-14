package app.sela.playback

import android.net.Uri
import okhttp3.OkHttpClient
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import org.schabi.newpipe.extractor.NewPipe
import org.schabi.newpipe.extractor.ServiceList
import org.schabi.newpipe.extractor.downloader.Downloader
import org.schabi.newpipe.extractor.downloader.Request
import org.schabi.newpipe.extractor.downloader.Response
import org.schabi.newpipe.extractor.search.SearchInfo
import org.schabi.newpipe.extractor.stream.StreamInfo
import org.schabi.newpipe.extractor.stream.StreamInfoItem
import java.io.IOException
import java.util.concurrent.TimeUnit

internal object Source {
    private val client = OkHttpClient.Builder().connectTimeout(20, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS).callTimeout(60, TimeUnit.SECONDS).build()
    init {
        NewPipe.init(object : Downloader() {
            override fun execute(request: Request): Response {
                val builder = okhttp3.Request.Builder().url(request.url())
                    .header("User-Agent", "Mozilla/5.0")
                request.headers().forEach { (name, values) ->
                    builder.removeHeader(name)
                    values.forEach { builder.addHeader(name, it) }
                }
                val body = request.dataToSend()?.toRequestBody()
                    ?: if (request.httpMethod() in listOf("POST", "PUT", "PATCH")) ByteArray(0).toRequestBody() else null
                builder.method(request.httpMethod(), body)
                client.newCall(builder.build()).execute().use { response ->
                    if (response.code == 429) throw IOException("YouTube membatasi permintaan (HTTP 429). Coba lagi nanti.")
                    val responseBody = response.body
                    val text = if (responseBody == null) "" else {
                        val limit = 8L * 1024 * 1024
                        if (responseBody.contentLength() > limit) throw IOException("Respons sumber melebihi 8 MiB")
                        val source = responseBody.source()
                        if (source.request(limit + 1)) throw IOException("Respons sumber melebihi 8 MiB")
                        source.readString(responseBody.contentType()?.charset(Charsets.UTF_8) ?: Charsets.UTF_8)
                    }
                    return Response(response.code, response.message, response.headers.toMultimap(),
                        text, response.request.url.toString())
                }
            }
        })
    }
    fun track(item: StreamInfoItem): JSONObject? {
        val id = runCatching { ServiceList.YouTube.streamLHFactory.getId(item.url) }.getOrNull() ?: return null
        if (!id.matches(Regex("[A-Za-z0-9_-]{11}"))) return null
        return JSONObject().put("id", id).put("title", item.name).put("channel", item.uploaderName)
    }
    fun search(query: String): JSONArray {
        require(query.isNotBlank() && query.length <= 500) { "Pencarian tidak valid" }
        val service = ServiceList.YouTube
        val info = try { SearchInfo.getInfo(service, service.searchQHFactory.fromQuery(query, listOf("videos"), "")) }
            catch (failure: Exception) { throw IOException("Pencarian sumber gagal (${failure.javaClass.simpleName})") }
        return JSONArray().also { rows -> info.relatedItems.filterIsInstance<StreamInfoItem>()
            .take(40).forEach { track(it)?.let(rows::put) } }
    }
    data class Resolved(val uri: String, val mime: String?, val artwork: String?, val related: List<JSONObject>)
    fun resolve(track: JSONObject): Resolved {
        val id = track.getString("id")
        if (Uri.parse(id).scheme == "content") return Resolved(id, null, null, emptyList())
        require(id.matches(Regex("[A-Za-z0-9_-]{11}"))) { "ID video tidak valid" }
        val info = try { StreamInfo.getInfo(ServiceList.YouTube, "https://www.youtube.com/watch?v=$id") }
            catch (failure: Exception) { throw IOException("Ekstraksi sumber gagal (${failure.javaClass.simpleName})") }
        val audio = info.audioStreams.filter { it.isUrl && it.content.startsWith("https://") }
            .maxByOrNull { it.averageBitrate }
        val uri = audio?.content ?: info.hlsUrl.takeIf { it.startsWith("https://") }
            ?: info.dashMpdUrl.takeIf { it.startsWith("https://") }
            ?: throw IOException("Tidak ada stream audio yang didukung")
        val mime = if (audio != null) audio.format?.mimeType else if (uri == info.hlsUrl) "application/x-mpegURL" else "application/dash+xml"
        return Resolved(uri, mime, info.thumbnails.firstOrNull()?.url,
            info.relatedItems.filterIsInstance<StreamInfoItem>().mapNotNull(::track))
    }
}

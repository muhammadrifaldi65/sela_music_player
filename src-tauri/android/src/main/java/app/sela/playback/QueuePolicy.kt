package app.sela.playback

internal fun queueIndex(current: Int, direction: Int, size: Int, repeat: String): Int? {
    require(direction == -1 || direction == 1)
    if (size == 0) return null
    val next = current + direction
    return if (repeat == "queue") Math.floorMod(next, size) else next.takeIf { it in 0 until size }
}

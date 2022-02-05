package net.alacrem.aegis.ui.main

import android.graphics.ImageDecoder
import android.graphics.drawable.Drawable
import java.nio.ByteBuffer
import java.sql.Date

class DevicePicture(val creationDate: Date, jpegData: ByteArray)
{
    val drawable: Drawable

    init {
        val src = ImageDecoder.createSource(ByteBuffer.wrap(jpegData))
        drawable = ImageDecoder.decodeDrawable(src)
    }
}

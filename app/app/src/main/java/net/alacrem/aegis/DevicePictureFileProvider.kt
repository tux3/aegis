package net.alacrem.aegis

import android.content.ContentProvider
import android.content.ContentValues
import android.content.UriMatcher
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.util.Log
import java.io.File
import java.io.FileNotFoundException
import java.util.*


class DevicePictureFileProvider: ContentProvider() {
    companion object {
        const val AUTHORITY = "net.alacrem.aegis.DevicePictureFileProvider"
    }

    private var uriMatcher: UriMatcher? = null

    override fun onCreate(): Boolean {
        uriMatcher = UriMatcher(UriMatcher.NO_MATCH)
        uriMatcher!!.addURI(AUTHORITY, "img/*", 1)
        return true
    }

    @Throws(FileNotFoundException::class)
    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor? {
        Log.i(TAG, "Provider openFile called with uri: '" + uri + "'." + uri.lastPathSegment)
        return when (uriMatcher!!.match(uri)) {
            1 -> {
                val fileLocation: String =
                    context!!.filesDir.absolutePath + File.separator + uri.lastPathSegment
                ParcelFileDescriptor.open(
                    File(fileLocation),
                    ParcelFileDescriptor.MODE_READ_ONLY
                )
            }
            else -> {
                Log.i(TAG, "Unsupported uri: '$uri'.")
                throw FileNotFoundException("Unsupported uri: $uri")
            }
        }
    }

    override fun update(
        uri: Uri,
        contentvalues: ContentValues?,
        s: String?,
        sarg: Array<String>?
    ): Int {
        return 0
    }

    override fun delete(uri: Uri, s: String?, `as`: Array<String?>?): Int {
        return 0
    }

    override fun insert(uri: Uri, contentvalues: ContentValues?): Uri? {
        return null
    }

    override fun getType(uri: Uri): String {
        return "image/jpeg"
    }

    override fun query(
        uri: Uri,
        projection: Array<String?>?,
        s: String?,
        as1: Array<String?>?,
        s1: String?
    ): Cursor {
        Log.i(TAG, "Provider query projection: "+Arrays.toString(projection))
        val fileLocation: String = context!!.filesDir.absolutePath + File.separator + uri.lastPathSegment
        val file = File(fileLocation)
        val time = System.currentTimeMillis()
        val c = MatrixCursor(
            arrayOf(
                "_id",
                "_data",
                "orientation",
                "mime_type",
                "datetaken",
                "_display_name"
            )
        )
        c.addRow(arrayOf<Any>(0, file, 0, "image/jpeg", time, uri.lastPathSegment!!))
        return c
    }

    override fun getStreamTypes(uri: Uri, mimeTypeFilter: String): Array<String?>? {
        return null
    }
}
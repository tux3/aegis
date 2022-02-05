package net.alacrem.aegis.ui.main

import android.content.Intent
import android.graphics.Bitmap
import android.graphics.drawable.BitmapDrawable
import android.graphics.drawable.Drawable
import android.net.Uri
import android.os.Bundle
import android.util.Log
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.recyclerview.widget.DividerItemDecoration
import androidx.recyclerview.widget.LinearLayoutManager
import net.alacrem.aegis.DevicePictureFileProvider
import net.alacrem.aegis.TAG
import net.alacrem.aegis.databinding.ActivityDevicePicturesListBinding
import net.alacrem.aegis.ui.DeviceSettingsActivity
import java.io.File
import java.io.FileOutputStream


@ExperimentalUnsignedTypes
class DevicePicturesListActivity : AppCompatActivity() {
    private lateinit var binding: ActivityDevicePicturesListBinding

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityDevicePicturesListBinding.inflate(layoutInflater)
        setContentView(binding.root)

        val deviceName = intent.getStringExtra("device_name")
        actionBar?.title = "$deviceName pictures"
        supportActionBar?.title = "$deviceName pictures"

        val adapter = DevicePictureAdapter(DeviceSettingsActivity.devicePictures, ::onPictureClicked)
        binding.picList.adapter = adapter
        binding.picList.layoutManager = LinearLayoutManager(applicationContext)
        binding.picList.addItemDecoration(DividerItemDecoration(applicationContext, DividerItemDecoration.VERTICAL))

    }

    private fun onPictureClicked(pic: Drawable) {
        if (pic is BitmapDrawable) {
            val bitmap = pic.bitmap
            val fileName = "aegis_capture.jpeg"
            try {
                val file = File(filesDir, fileName)
                val fos = FileOutputStream(file)
                bitmap.compress(Bitmap.CompressFormat.JPEG, 100, fos)
                fos.flush()
                fos.close()
            } catch (e: Exception) {
                Log.e(TAG, "Failed to save picture: $e")
                Toast.makeText(applicationContext, "Failed to save picture", Toast.LENGTH_LONG).show()
                return
            }

            val uri = Uri.parse("content://" + DevicePictureFileProvider.AUTHORITY + File.separator + "img" + File.separator + fileName)
            val intent = Intent()
            intent.action = Intent.ACTION_VIEW
            intent.type = "image/jpeg"
            intent.data = uri
            intent.putExtra(Intent.EXTRA_TEXT, "Aegis captured picture")
            intent.putExtra(Intent.EXTRA_STREAM, uri)
            intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            startActivity(intent)
        }
    }
}
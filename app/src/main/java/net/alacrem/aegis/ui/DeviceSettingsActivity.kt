package net.alacrem.aegis.ui

import android.R
import android.content.Intent
import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import net.alacrem.aegis.databinding.ActivityDeviceSettingsBinding
import net.alacrem.aegis.ui.main.MainActivity
import uniffi.client.RootKeys
import androidx.core.app.NavUtils

import android.view.MenuItem




@ExperimentalUnsignedTypes
class DeviceSettingsActivity : AppCompatActivity() {
    private lateinit var binding: ActivityDeviceSettingsBinding
    private lateinit var keys: RootKeys
    private lateinit var deviceName: String

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        supportActionBar?.setDisplayHomeAsUpEnabled(true)

        var maybeKeys: RootKeys? = null
        var maybeDevName: String? = null
        if (savedInstanceState != null) {
            maybeDevName = savedInstanceState.getString("device_name")
            val savedKeyData = savedInstanceState.getByteArray("keys")
            if (savedKeyData != null) {
                maybeKeys = RootKeys.fromBytes(savedKeyData.toUByteArray().asList())
            }
        }
        if (intent != null) {
            maybeDevName = intent.getStringExtra("device_name")
            val data = intent.getByteArrayExtra("keys")
            if (data != null)
                maybeKeys = RootKeys.fromBytes(data.toUByteArray().asList())
        }
        if (maybeKeys == null || maybeDevName == null) {
            startActivity(Intent(this, MainActivity::class.java))
            finish()
            return
        } else {
            keys = maybeKeys
            deviceName = maybeDevName
        }

        actionBar?.title = "$deviceName device settings"
        supportActionBar?.title = "$deviceName device settings"

        // TODO: Actually load current status
        // TODO: Update status in background when widget triggered

        binding = ActivityDeviceSettingsBinding.inflate(layoutInflater)
        setContentView(binding.root)
    }

    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        return when (item.itemId) {
            R.id.home -> {
                NavUtils.navigateUpFromSameTask(this)
                true
            }
            else -> super.onOptionsItemSelected(item)
        }
    }
}
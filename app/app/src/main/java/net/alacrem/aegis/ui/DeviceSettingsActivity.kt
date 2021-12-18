package net.alacrem.aegis.ui

import android.content.Intent
import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import net.alacrem.aegis.R
import net.alacrem.aegis.databinding.ActivityDeviceSettingsBinding
import net.alacrem.aegis.ui.main.MainActivity
import androidx.core.app.NavUtils

import android.view.MenuItem
import android.view.View
import android.widget.Toast
import androidx.appcompat.widget.SwitchCompat
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import net.alacrem.aegis.defaultClientConfig
import uniffi.client.*


@ExperimentalUnsignedTypes
class DeviceSettingsActivity : AppCompatActivity() {
    private lateinit var binding: ActivityDeviceSettingsBinding
    private lateinit var keys: RootKeys
    private lateinit var deviceName: String
    private lateinit var client: AdminClientFfi

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
        client = AdminClientFfi(defaultClientConfig(), keys)

        actionBar?.title = "$deviceName device settings"
        supportActionBar?.title = "$deviceName device settings"

        binding = ActivityDeviceSettingsBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.lockBtn.setOnClickListener {
            updateStatusSync(SetStatusArg(deviceName, vtLocked = true, sshLocked = true))
        }
        binding.unlockBtn.setOnClickListener {
            updateStatusSync(SetStatusArg(deviceName, vtLocked = false, sshLocked = false))
        }
        refreshStatus()
    }

    private fun refreshStatus() {
        updateStatusSync(SetStatusArg(deviceName, null, null))
    }

    private fun updateStatusSync(statusChange: SetStatusArg) {
        disableUi()
        binding.settingsLoading.visibility = View.VISIBLE
        binding.settingsLoadingBg.visibility = View.VISIBLE
        lifecycleScope.launch(Dispatchers.IO) {
            val newStatus = try {
                client.setStatus(statusChange)
            } catch (e: FfiException) {
                withContext(Dispatchers.Main) {
                    Toast.makeText(applicationContext, "Failed to set status", Toast.LENGTH_LONG).show()
                    binding.settingsLoading.visibility = View.GONE
                    binding.settingsLoadingBg.visibility = View.GONE
                    enableUi()
                }
                return@launch
            }
            withContext(Dispatchers.Main) {
                applyStatusReply(newStatus)
            }
        }
    }

    private fun disableUi() {
        binding.settingsVlayout.isEnabled = false
        binding.lockBtn.isEnabled = false
        binding.unlockBtn.isEnabled = false
        binding.vtLock.setOnCheckedChangeListener(null)
        binding.vtLock.isEnabled = false
        binding.sshLock.setOnCheckedChangeListener(null)
        binding.sshLock.isEnabled = false
    }

    private fun enableUi() {
        binding.lockBtn.isEnabled = true
        binding.unlockBtn.isEnabled = true
        binding.vtLock.setOnCheckedChangeListener { _, isChecked ->
            updateStatusSync(SetStatusArg(deviceName, vtLocked = isChecked, null))
        }
        binding.vtLock.isEnabled = true
        binding.sshLock.setOnCheckedChangeListener { _, isChecked ->
            updateStatusSync(SetStatusArg(deviceName, null, sshLocked = isChecked))
        }
        binding.sshLock.isEnabled = true
        binding.settingsVlayout.isEnabled = true
    }

    private fun applyStatusReply(status: StatusReply) {
        setSwitch(binding.vtLock, status.vtLocked)
        setSwitch(binding.sshLock, status.sshLocked)
        enableUi()
        binding.settingsLoading.visibility = View.GONE
        binding.settingsLoadingBg.visibility = View.GONE
        binding.settingsVlayout.isEnabled = true
    }

    private fun setSwitch(switch: SwitchCompat, state: Boolean) {
        switch.isChecked = state
        switch.jumpDrawablesToCurrentState()
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
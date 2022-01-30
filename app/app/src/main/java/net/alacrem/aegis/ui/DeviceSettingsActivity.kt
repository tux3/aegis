package net.alacrem.aegis.ui

import android.app.AlertDialog
import android.content.DialogInterface
import android.content.Intent
import android.os.Bundle
import android.view.MenuItem
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.SwitchCompat
import androidx.core.app.NavUtils
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import net.alacrem.aegis.ClientBuilder
import net.alacrem.aegis.R
import net.alacrem.aegis.databinding.ActivityDeviceSettingsBinding
import net.alacrem.aegis.ui.main.MainActivity
import uniffi.client.*
import java.sql.Date
import java.text.SimpleDateFormat


@ExperimentalUnsignedTypes
class DeviceSettingsActivity : AppCompatActivity() {
    private lateinit var binding: ActivityDeviceSettingsBinding
    private lateinit var deviceName: String
    private lateinit var client: AdminClientFfi

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        supportActionBar?.setDisplayHomeAsUpEnabled(true)

        val clientBuilder = try {
            ClientBuilder(savedInstanceState, intent)
        } catch (e: IllegalArgumentException) {
            startActivity(Intent(this, MainActivity::class.java))
            finish()
            return
        }
        deviceName = clientBuilder.deviceName
        client = clientBuilder.build()

        actionBar?.title = "$deviceName device settings"
        supportActionBar?.title = "$deviceName device settings"

        binding = ActivityDeviceSettingsBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.lockBtn.setOnClickListener {
            updateStatusSync(SetStatusArg(deviceName, vtLocked = true, sshLocked = true, drawDecoy = null))
        }
        binding.unlockBtn.setOnClickListener {
            updateStatusSync(SetStatusArg(deviceName, vtLocked = false, sshLocked = false, drawDecoy = null))
        }
        binding.unlinkBtn.setOnClickListener {
            AlertDialog.Builder(this)
                .setTitle("Really unlink?")
                .setMessage("Do you really want to remove this device?")
                .setIcon(android.R.drawable.ic_dialog_alert)
                .setPositiveButton(android.R.string.ok
                ) { _, _ ->
                    unlinkDevice()
                }
                .setNegativeButton(android.R.string.cancel, null).show()

        }
        binding.clearPicsStorageBtn.setOnClickListener {
            AlertDialog.Builder(this)
                .setTitle("Really erase pictures?")
                .setMessage("Do you really want to erase stored camera pictures?")
                .setIcon(android.R.drawable.ic_dialog_alert)
                .setPositiveButton(android.R.string.ok
                ) { _, _ ->
                    clearPictures()
                }
                .setNegativeButton(android.R.string.cancel, null).show()

        }
        refreshStatus()
    }

    private fun clearPictures() {
        disableUi()
        binding.settingsLoading.visibility = View.VISIBLE
        binding.settingsLoadingBg.visibility = View.VISIBLE
        lifecycleScope.launch(Dispatchers.IO) {
            client.deleteDeviceCameraPictures(deviceName)
            withContext(Dispatchers.Main) {
                Toast.makeText(applicationContext, "Cleared device pictures storage", Toast.LENGTH_SHORT).show()
                finish()
            }
        }
    }

    private fun unlinkDevice() {
        disableUi()
        binding.settingsLoading.visibility = View.VISIBLE
        binding.settingsLoadingBg.visibility = View.VISIBLE
        lifecycleScope.launch(Dispatchers.IO) {
            client.deleteRegistered(deviceName)
            withContext(Dispatchers.Main) {
                Toast.makeText(applicationContext, "Unlinked device '$deviceName'", Toast.LENGTH_SHORT).show()
                finish()
            }
        }
    }

    private fun refreshStatus() {
        updateStatusSync(SetStatusArg(deviceName, null, null, null))
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
        binding.drawDecoy.setOnCheckedChangeListener(null)
        binding.drawDecoy.isEnabled = false
        binding.unlinkBtn.isEnabled = false
        binding.clearPicsStorageBtn.isEnabled = false
    }

    private fun enableUi() {
        binding.lockBtn.isEnabled = true
        binding.unlockBtn.isEnabled = true
        binding.vtLock.setOnCheckedChangeListener { _, isChecked ->
            updateStatusSync(SetStatusArg(deviceName, vtLocked = isChecked, null, null))
        }
        binding.vtLock.isEnabled = true
        binding.sshLock.setOnCheckedChangeListener { _, isChecked ->
            updateStatusSync(SetStatusArg(deviceName, null, sshLocked = isChecked, null))
        }
        binding.sshLock.isEnabled = true
        binding.drawDecoy.setOnCheckedChangeListener { _, isChecked ->
            updateStatusSync(SetStatusArg(deviceName, null, null, drawDecoy = isChecked))
        }
        binding.drawDecoy.isEnabled = true
        binding.unlinkBtn.isEnabled = true
        binding.clearPicsStorageBtn.isEnabled = true
        binding.settingsVlayout.isEnabled = true
    }

    private fun applyStatusReply(status: StatusReply) {
        setSwitch(binding.vtLock, status.vtLocked)
        setSwitch(binding.sshLock, status.sshLocked)
        setSwitch(binding.drawDecoy, status.drawDecoy)
        val dateFormat = SimpleDateFormat("dd-MM-yyyy HH:mm:ss")
        binding.lastStatusChangeLbl.text = dateFormat.format(Date(status.updatedAtTimestamp.toLong() * 1000))
        binding.websocketStatusLbl.text = if (status.isConnected) "Connected" else "Disconnected"
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
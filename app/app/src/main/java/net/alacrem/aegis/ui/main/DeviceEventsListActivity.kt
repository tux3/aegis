package net.alacrem.aegis.ui.main

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.recyclerview.widget.DividerItemDecoration
import androidx.recyclerview.widget.LinearLayoutManager
import net.alacrem.aegis.databinding.ActivityDeviceEventsListBinding
import net.alacrem.aegis.ui.DeviceSettingsActivity


@ExperimentalUnsignedTypes
class DeviceEventsListActivity : AppCompatActivity() {
    private lateinit var binding: ActivityDeviceEventsListBinding

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityDeviceEventsListBinding.inflate(layoutInflater)
        setContentView(binding.root)

        val deviceName = intent.getStringExtra("device_name")
        actionBar?.title = "$deviceName event log"
        supportActionBar?.title = "$deviceName event log"

        val deviceEvents = DeviceSettingsActivity.deviceEvents
        val adapter = DeviceEventAdapter(deviceEvents)
        binding.eventList.adapter = adapter
        binding.eventList.layoutManager = LinearLayoutManager(applicationContext)
        binding.eventList.addItemDecoration(DividerItemDecoration(applicationContext, DividerItemDecoration.VERTICAL))
        binding.eventList.scrollToPosition(deviceEvents.size-1)
    }
}
package net.alacrem.aegis.ui.main

import android.annotation.SuppressLint
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView
import net.alacrem.aegis.R
import uniffi.client.DeviceEvent
import java.sql.Date
import java.text.SimpleDateFormat


class DeviceEventAdapter(private val events: Array<DeviceEvent>) : RecyclerView.Adapter<DeviceEventAdapter.ViewHolder>() {

    // Provide a direct reference to each of the views within a data item
    // Used to cache the views within the item layout for fast access
    inner class ViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
        // Your holder should contain and initialize a member variable
        // for any view that will be set as you render a row
        val createdAtView: TextView = itemView.findViewById(R.id.created_at)
        val levelView: TextView = itemView.findViewById(R.id.log_level)
        val messageView: TextView = itemView.findViewById(R.id.log_message)
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): DeviceEventAdapter.ViewHolder {
        val context = parent.context
        val inflater = LayoutInflater.from(context)
        val view = inflater.inflate(R.layout.item_device_event, parent, false)
        return ViewHolder(view)
    }

    // Involves populating data into the item through holder
    @SuppressLint("SimpleDateFormat")
    override fun onBindViewHolder(viewHolder: DeviceEventAdapter.ViewHolder, position: Int) {
        val event = events[position]
        val dateFormat = SimpleDateFormat("dd-MM-yyyy\nHH:mm:ss")
        viewHolder.createdAtView.text = dateFormat.format(Date(event.timestamp.toLong() * 1000))
        viewHolder.levelView.text = event.level.name
        viewHolder.messageView.text = event.message
    }

    // Returns the total count of items in the list
    override fun getItemCount(): Int {
        return events.size
    }
}
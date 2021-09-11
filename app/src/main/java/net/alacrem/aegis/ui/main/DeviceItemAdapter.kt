package net.alacrem.aegis.ui.main

import android.annotation.SuppressLint
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView
import net.alacrem.aegis.R
import uniffi.client.RegisteredDevice
import android.widget.Toast




class DeviceItemAdapter(private val onClick: (devName: String) -> Unit) : RecyclerView.Adapter<DeviceItemAdapter.ViewHolder>() {
    private var devices: List<RegisteredDevice> = ArrayList()

    // Provide a direct reference to each of the views within a data item
    // Used to cache the views within the item layout for fast access
    inner class ViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
        // Your holder should contain and initialize a member variable
        // for any view that will be set as you render a row
        val nameTextView = itemView.findViewById<TextView>(R.id.dev_name)

        init {
            itemView.setOnClickListener {
                if (adapterPosition != RecyclerView.NO_POSITION) {
                    onClick(nameTextView.text.toString())
                }
            }
        }
    }

    @SuppressLint("NotifyDataSetChanged")
    fun updateContents(devices: List<RegisteredDevice>) {
        this.devices = devices
        notifyDataSetChanged()
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): DeviceItemAdapter.ViewHolder {
        val context = parent.context
        val inflater = LayoutInflater.from(context)
        val view = inflater.inflate(R.layout.item_device, parent, false)
        return ViewHolder(view)
    }

    // Involves populating data into the item through holder
    override fun onBindViewHolder(viewHolder: DeviceItemAdapter.ViewHolder, position: Int) {
        val dev = devices[position]
        viewHolder.nameTextView.text = dev.name
    }

    // Returns the total count of items in the list
    override fun getItemCount(): Int {
        return devices.size
    }
}
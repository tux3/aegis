package net.alacrem.aegis.ui.main

import android.annotation.SuppressLint
import android.graphics.drawable.Drawable
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView
import net.alacrem.aegis.R
import java.text.SimpleDateFormat


class DevicePictureAdapter(private val pictures: Array<DevicePicture>, private val onClick: (picture: Drawable) -> Unit) : RecyclerView.Adapter<DevicePictureAdapter.ViewHolder>() {

    // Provide a direct reference to each of the views within a data item
    // Used to cache the views within the item layout for fast access
    inner class ViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
        // Your holder should contain and initialize a member variable
        // for any view that will be set as you render a row
        val createdAtView = itemView.findViewById<TextView>(R.id.created_at)
        val pictureView = itemView.findViewById<ImageView>(R.id.picture)

        init {
            itemView.setOnClickListener {
                if (adapterPosition != RecyclerView.NO_POSITION) {
                    onClick(pictureView.drawable)
                }
            }
        }
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): DevicePictureAdapter.ViewHolder {
        val context = parent.context
        val inflater = LayoutInflater.from(context)
        val view = inflater.inflate(R.layout.item_device_picture, parent, false)
        return ViewHolder(view)
    }

    // Involves populating data into the item through holder
    @SuppressLint("SimpleDateFormat")
    override fun onBindViewHolder(viewHolder: DevicePictureAdapter.ViewHolder, position: Int) {
        val pic = pictures[position]
        val dateFormat = SimpleDateFormat("dd-MM-yyyy HH:mm:ss")
        viewHolder.createdAtView.text = dateFormat.format(pic.creationDate)
        viewHolder.pictureView.setImageDrawable(pic.drawable)
    }

    // Returns the total count of items in the list
    override fun getItemCount(): Int {
        return pictures.size
    }
}
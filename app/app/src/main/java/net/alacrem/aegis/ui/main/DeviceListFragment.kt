package net.alacrem.aegis.ui.main

import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Toast
import androidx.fragment.app.Fragment
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import net.alacrem.aegis.R
import net.alacrem.aegis.databinding.FragmentMainBinding
import net.alacrem.aegis.defaultClientConfig
import uniffi.client.AdminClientFfi
import uniffi.client.RootKeys
import java.lang.IllegalArgumentException
import androidx.recyclerview.widget.DividerItemDecoration
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import net.alacrem.aegis.ROOT_KEYS_FILE
import net.alacrem.aegis.TAG
import net.alacrem.aegis.ui.DeviceSettingsActivity
import net.alacrem.aegis.ui.login.LoginActivity
import uniffi.client.FfiException
import java.io.File


/**
 * A placeholder fragment containing a simple view.
 * Fragments should have a default constructor, otherwise we may crash at runtime
 */
@ExperimentalUnsignedTypes
class DeviceListFragment(val parent: SectionsPagerAdapter) : Fragment() {
    private lateinit var devicePageViewModel: DevicePageViewModel
    private lateinit var kind: TabKind
    private lateinit var keys: RootKeys
    private var _binding: FragmentMainBinding? = null
    private var adapter: RecyclerView.Adapter<*>? = null

    // This property is only valid between onCreateView and
    // onDestroyView.
    private val binding get() = _binding!!

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        kind = TabKind.values()[arguments?.getInt(ARG_KIND_IDX)!!]
        keys = RootKeys.fromBytes(arguments?.getByteArray(ARG_KEYS)!!.toUByteArray().asList())
    }

    private fun loadDeviceListAsync() {
        binding.swipeContainer.isRefreshing = true
        lifecycleScope.launch(Dispatchers.IO) {
            try {
                val client = AdminClientFfi(defaultClientConfig(), keys)
                if (kind == TabKind.REGISTERED) {
                        val devList = client.listRegistered()
                        withContext(Dispatchers.Main) {
                            if (adapter != null)
                                (adapter as DeviceItemAdapter).updateContents(devList)
                            binding.swipeContainer.isRefreshing = false
                        }
                } else if (kind == TabKind.PENDING) {
                    val devList = client.listPending()
                    withContext(Dispatchers.Main) {
                        if (adapter != null)
                            (adapter as PendingDeviceItemAdapter).updateContents(devList)
                        binding.swipeContainer.isRefreshing = false
                    }
                }
            } catch (e: FfiException) {
                if (e.message?.contains("HTTP error 403") == true) {
                    Log.e(TAG, "Caught FFI HTTP 403 exception, deleting saved keys")
                    val context = requireContext()
                    File(context.filesDir, ROOT_KEYS_FILE).delete()
                    startActivity(Intent(context, LoginActivity::class.java))
                    activity?.finish()
                }
            }
        }
    }

    private fun onDeviceClicked(name: String) {
        val intent = Intent(context, DeviceSettingsActivity::class.java)
        intent.putExtra("keys", keys.toBytes().toUByteArray().toByteArray())
        intent.putExtra("device_name", name)
        startActivity(intent)
    }

    private fun onPendingClicked(name: String) {
        // TODO: Load pending activity
        Toast.makeText(context, name, Toast.LENGTH_SHORT).show()
    }

    private fun onPendingConfirmClicked(name: String) {
        lifecycleScope.launch(Dispatchers.IO) {
            val client = AdminClientFfi(defaultClientConfig(), keys)
            client.confirmPending(name)
            withContext(Dispatchers.Main) {
                Toast.makeText(context, "Confirmed device '$name'", Toast.LENGTH_SHORT).show()
                loadDeviceListAsync()
            }
        }
    }

    override fun onResume() {
        super.onResume()
        loadDeviceListAsync()
    }

    override fun onCreateView(
        inflater: LayoutInflater, container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View? {
        _binding = FragmentMainBinding.inflate(inflater, container, false)
        val root = binding.root

        if (kind == TabKind.REGISTERED) {
            binding.sectionLabel.text = getString(R.string.label_registered_dev)
            adapter = DeviceItemAdapter(::onDeviceClicked)
        } else if (kind == TabKind.PENDING) {
            binding.sectionLabel.text = getString(R.string.label_pending_dev)
            adapter = PendingDeviceItemAdapter(::onPendingClicked, ::onPendingConfirmClicked)
        } else {
            throw IllegalArgumentException("Invalid TabKind")
        }
        loadDeviceListAsync()

        binding.swipeContainer.setOnRefreshListener {
            loadDeviceListAsync()
        }

        binding.devList.adapter = adapter
        binding.devList.layoutManager = LinearLayoutManager(context)
        binding.devList.addItemDecoration(DividerItemDecoration(context, DividerItemDecoration.VERTICAL))
        return root
    }

    companion object {
        private const val ARG_KIND_IDX = "tab_kind"
        private const val ARG_KEYS = "root_keys"

        @JvmStatic
        fun newInstance(parent: SectionsPagerAdapter, kind: TabKind, keys: RootKeys): DeviceListFragment {
            return DeviceListFragment(parent).apply {
                arguments = Bundle().apply {
                    putInt(ARG_KIND_IDX, kind.ordinal)
                    putByteArray(ARG_KEYS, keys.toBytes().toUByteArray().toByteArray())
                }
            }
        }
    }

    override fun onDestroyView() {
        super.onDestroyView()
        _binding = null
    }
}
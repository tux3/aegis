package net.alacrem.aegis

import android.content.Intent
import android.os.Bundle
import uniffi.client.AdminClientFfi
import uniffi.client.RootKeys

@ExperimentalUnsignedTypes
class ClientBuilder(savedState: Bundle?, intent: Intent?) {
    private val keys: RootKeys
    val deviceName: String

    init {
        var maybeKeys: RootKeys? = null
        var maybeDevName: String? = null
        if (savedState != null) {
            maybeDevName = savedState.getString("device_name")
            val savedKeyData = savedState.getByteArray("keys")
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
            throw IllegalArgumentException("Could not find keys or device name for building client")
        } else {
            keys = maybeKeys
            deviceName = maybeDevName
        }
    }

    fun build(): AdminClientFfi {
        return AdminClientFfi(defaultClientConfig(), keys)
    }
}
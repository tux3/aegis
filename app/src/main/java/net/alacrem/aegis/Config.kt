package net.alacrem.aegis

import uniffi.client.ClientConfig

const val TAG = "aegisd"

const val ROOT_KEYS_FILE = "root.keys"

// TODO: This is the test/dev key! Replace by a prod key later...
const val ROOT_SIG_PUBKEY = "fdq2MVHvsmzSRWy9tR9FHj-o1Ws7buZ5RHDLm5ljFfU"
const val SERVER_ADDR = "10.0.2.2:8080"
const val SERVER_USE_TLS = false

//const val ROOT_SIG_PUBKEY = "ut1lMipIOC5EeoJ_Fj7LeYmr6Rs3Qi4e3EO2SkKwxrQ"
//const val SERVER_ADDR = "alacrem.net/aegis/"
//const val SERVER_USE_TLS = true

fun defaultClientConfig(): ClientConfig {
    return ClientConfig(SERVER_ADDR, SERVER_USE_TLS, false)
}
package net.alacrem.aegis.ui.login

import uniffi.client.RootKeys

/**
 * Authentication result : success (user details) or error message.
 */
data class LoginResult(
    val success: RootKeys? = null,
    val error: Int? = null
)
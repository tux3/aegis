package net.alacrem.aegis.ui.login

import androidx.lifecycle.LiveData
import androidx.lifecycle.MutableLiveData
import androidx.lifecycle.ViewModel

import net.alacrem.aegis.R
import net.alacrem.aegis.ROOT_SIG_PUBKEY
import uniffi.client.RootKeys

class LoginViewModel : ViewModel() {
    private val _loginResult = MutableLiveData<LoginResult>()
    val loginResult: LiveData<LoginResult> = _loginResult

    fun login(password: String) {
        val keys = RootKeys.derive(password)
        if (keys.matchesSerializesPubkey(ROOT_SIG_PUBKEY)) {
            _loginResult.value = LoginResult(success = keys)
        } else {
            _loginResult.value = LoginResult(error = R.string.login_failed)
        }
    }
}
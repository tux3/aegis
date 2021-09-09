package net.alacrem.aegis.ui.login

import androidx.lifecycle.LiveData
import androidx.lifecycle.MutableLiveData
import androidx.lifecycle.ViewModel

import net.alacrem.aegis.R
import uniffi.client.RootKeys

// TODO: This is the test/dev key! Replace by a prod key later...
const val ROOT_SIG_PUBKEY = "fdq2MVHvsmzSRWy9tR9FHj-o1Ws7buZ5RHDLm5ljFfU"

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
package net.alacrem.aegis.ui.login

import androidx.lifecycle.LiveData
import androidx.lifecycle.MutableLiveData
import androidx.lifecycle.ViewModel

import net.alacrem.aegis.R

class LoginViewModel : ViewModel() {
    private val _loginResult = MutableLiveData<LoginResult>()
    val loginResult: LiveData<LoginResult> = _loginResult

    fun login(password: String) {
        // TODO: Login

        if (password.isNotEmpty()) {
            _loginResult.value =
                LoginResult(success = LoggedInUserView(displayName = "Placeholder"))
        } else {
            _loginResult.value = LoginResult(error = R.string.login_failed)
        }
    }
}
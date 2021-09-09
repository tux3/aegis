package net.alacrem.aegis.ui.login

import android.app.Activity
import androidx.lifecycle.Observer
import androidx.lifecycle.ViewModelProvider
import android.os.Bundle
import android.util.Log
import androidx.annotation.StringRes
import androidx.appcompat.app.AppCompatActivity
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.Toast
import net.alacrem.aegis.ROOT_KEYS_FILE
import net.alacrem.aegis.TAG
import net.alacrem.aegis.databinding.ActivityLoginBinding
import uniffi.client.RootKeys
import java.io.File
import java.io.FileNotFoundException

class LoginActivity : AppCompatActivity() {

    private lateinit var loginViewModel: LoginViewModel
    private lateinit var binding: ActivityLoginBinding

    private fun loadRootKeys(): RootKeys {
        val data = File(filesDir, ROOT_KEYS_FILE).readBytes()
        return RootKeys.fromBytes(data.toUByteArray().asList())
    }

    private fun saveRootKeys(keys: RootKeys) {
        val data = keys.toBytes().toUByteArray().toByteArray()
        File(filesDir, ROOT_KEYS_FILE).writeBytes(data)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        try {
            val keys = loadRootKeys()
            Log.i(TAG, "Loaded root keys from file")

            setResult(Activity.RESULT_OK)
            finish()
        } catch (e: FileNotFoundException) {
            // Alright, no problem
        }

        binding = ActivityLoginBinding.inflate(layoutInflater)
        setContentView(binding.root)

        val password = binding.password
        val login = binding.login
        val loading = binding.loading

        loginViewModel = ViewModelProvider(this).get(LoginViewModel::class.java)

        loginViewModel.loginResult.observe(this@LoginActivity, Observer {
            val loginResult = it ?: return@Observer

            loading.visibility = View.GONE
            if (loginResult.error != null) {
                showLoginFailed(loginResult.error)
            }
            if (loginResult.success != null) {
                val keys = loginResult.success
                saveRootKeys(keys)
                setResult(Activity.RESULT_OK)
                finish()
            }
        })

        password.apply {
            setOnEditorActionListener { _, actionId, _ ->
                when (actionId) {
                    EditorInfo.IME_ACTION_DONE ->
                        loginViewModel.login(
                            password.text.toString()
                        )
                }
                false
            }

            login.setOnClickListener {
                loading.visibility = View.VISIBLE
                loginViewModel.login(password.text.toString())
            }
        }
    }

    private fun showLoginFailed(@StringRes errorString: Int) {
        Toast.makeText(applicationContext, errorString, Toast.LENGTH_SHORT).show()
    }
}

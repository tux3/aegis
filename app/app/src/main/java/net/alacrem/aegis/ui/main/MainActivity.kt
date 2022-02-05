package net.alacrem.aegis.ui.main

import android.content.Intent
import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.viewpager.widget.ViewPager
import com.google.android.material.tabs.TabLayout
import net.alacrem.aegis.databinding.ActivityMainBinding
import net.alacrem.aegis.ui.login.LoginActivity
import uniffi.client.RootKeys

@ExperimentalUnsignedTypes
class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var keys: RootKeys

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        var maybeKeys: RootKeys? = null
        if (savedInstanceState != null) {
            val savedKeyData = savedInstanceState.getByteArray("keys")
            if (savedKeyData != null) {
                maybeKeys = RootKeys.fromBytes(savedKeyData.toUByteArray().asList())
            }
        }

        if (intent != null) {
            val data = intent.getByteArrayExtra("keys")
            if (data != null)
                maybeKeys = RootKeys.fromBytes(data.toUByteArray().asList())
        }

        if (maybeKeys == null) {
            startActivity(Intent(this, LoginActivity::class.java))
            finish()
            return
        } else {
            keys = maybeKeys
        }
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)

        val sectionsPagerAdapter = SectionsPagerAdapter(this, keys, supportFragmentManager)
        val viewPager: ViewPager = binding.viewPager
        viewPager.adapter = sectionsPagerAdapter
        val tabs: TabLayout = binding.tabs
        tabs.setupWithViewPager(viewPager)

    }
}
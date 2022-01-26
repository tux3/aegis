package net.alacrem.aegis.ui.main

import android.content.Context
import androidx.fragment.app.Fragment
import androidx.fragment.app.FragmentManager
import androidx.fragment.app.FragmentPagerAdapter
import net.alacrem.aegis.R
import uniffi.client.RootKeys

private val TAB_TITLES = arrayOf(
    R.string.tab_registered,
    R.string.tab_pending
)

enum class TabKind {
    REGISTERED, PENDING
}

/**
 * A [FragmentPagerAdapter] that returns a fragment corresponding to
 * one of the sections/tabs/pages.
 */
@ExperimentalUnsignedTypes
class SectionsPagerAdapter(
    private val context: Context,
    private val keys: RootKeys,
    fm: FragmentManager
) :
    FragmentPagerAdapter(fm, BEHAVIOR_RESUME_ONLY_CURRENT_FRAGMENT) {

    override fun getItem(position: Int): Fragment {
        // getItem is called to instantiate the fragment for the given page.
        // Return a PlaceholderFragment (defined as a static inner class below).
        return DeviceListFragment.newInstance(this, TabKind.values()[position], keys)
    }

    override fun getPageTitle(position: Int): CharSequence? {
        return context.resources.getString(TAB_TITLES[position])
    }

    override fun getCount(): Int {
        // Show 2 total pages.
        return 2
    }
}
plugins {
    // NOTE: The plugin{} blocks is not normal, it can't run arbitrary code or access outer scope
    // This allows Gradle to treat the plugin block as side-effect free.
    // (There is an exception for version catalogs, which works without warnings on Gradle >= 8.1)
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.jetbrains.kotlin.android) apply false
}

tasks.register<Delete>("clean") {
    delete(rootProject.buildDir)
}
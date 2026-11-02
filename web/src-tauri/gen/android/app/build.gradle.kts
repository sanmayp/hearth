import java.util.Properties
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

android {
    compileSdk = 37
    namespace = "com.hearth.app"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "com.hearth.app"
        minSdk = 24
        targetSdk = 37
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
    }
    buildTypes {
        getByName("debug") {
            applicationIdSuffix = ".debug"
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {
                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }
        getByName("release") {
            optimization {
               enable = true
            }
            proguardFiles(
                *fileTree(".") {
                  include("**/*.pro")
                  exclude("build/**")
                }.files.toTypedArray()
            )
            // Release key comes from env (publish script sets all four).
            // Missing vars fail a RELEASE build: a release must never
            // sign with the debug key, or later updates stop working.
            // Debug builds skip this block: Gradle configures every
            // build type on every run, so an unconditional throw here
            // would break debug builds too.
            val wantsRelease = gradle.startParameter.taskNames.any { t ->
                t.contains("Release", ignoreCase = true)
            };
            val envStore = System.getenv("HEARTH_KEYSTORE_PATH");
            val envStorePass = System.getenv("HEARTH_KEYSTORE_PASSWORD");
            val envAlias = System.getenv("HEARTH_KEY_ALIAS");
            val envKeyPass = System.getenv("HEARTH_KEY_PASSWORD");
            val envOk = envStore != null && envStorePass != null && envAlias != null && envKeyPass != null;
            if (wantsRelease && !envOk) {
                throw GradleException("release keystore env missing: HEARTH_KEYSTORE_PATH/PASSWORD, HEARTH_KEY_ALIAS/PASSWORD (see scripts/publish-apk.sh)");
            }
            if (envOk) {
                signingConfigs {
                    create("release") {
                        storeFile = file(envStore)
                        storePassword = envStorePass
                        keyAlias = envAlias
                        keyPassword = envKeyPass
                    }
                }
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    buildFeatures {
        buildConfig = true
    }
}

kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_1_8
    }
}

rust {
    rootDirRel = "../../../"
}

dependencies {
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = file("tauri.build.gradle.kts"))

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "jp.yatate.ime"
    compileSdk = 34

    defaultConfig {
        applicationId = "jp.yatate.ime"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "0.1"
        // 実機（arm64）と emulator（x86_64）の二つだけ。
        // 32bit は今どき要らず、入れるだけ APK が太る。
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }

    sourceSets["main"].java.srcDirs("src/main/kotlin")
    // cargo-ndk が libyatate_android.so をここへ置く（CI が gradle の前に回す）
    sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")
    sourceSets["test"].java.srcDirs("src/test/kotlin")

    lint {
        abortOnError = false
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
}

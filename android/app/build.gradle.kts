import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

val repoRoot = rootProject.projectDir.parentFile
val generatedLicenseAssets = layout.buildDirectory.dir("generated/licenseAssets")
val isWindows = System.getProperty("os.name").orEmpty().startsWith("Windows")
val ksFile = rootProject.file("keystore.properties")
val ks = Properties()
if (ksFile.exists()) ksFile.inputStream().use { ks.load(it) }

fun signingValue(property: String, environment: String): String? =
    ks.getProperty(property)?.takeIf { it.isNotBlank() }
        ?: System.getenv(environment)?.takeIf { it.isNotBlank() }

val releaseStorePath = signingValue("storeFile", "QUARK_ANDROID_STORE_FILE")
val releaseStoreFile =
    releaseStorePath?.let { path ->
        File(path).let { if (it.isAbsolute) it else rootProject.file(it) }
    }
val releaseStorePassword = signingValue("storePassword", "QUARK_ANDROID_STORE_PASSWORD")
val releaseKeyAlias = signingValue("keyAlias", "QUARK_ANDROID_KEY_ALIAS")
val releaseKeyPassword = signingValue("keyPassword", "QUARK_ANDROID_KEY_PASSWORD")
val hasReleaseKeystore =
    releaseStoreFile?.isFile == true &&
        listOf(releaseStorePassword, releaseKeyAlias, releaseKeyPassword).all { it != null }

fun pythonCmd(): List<String> =
    if (isWindows) listOf("py", "-3") else listOf("python3")

fun alignSoTree(dir: File) {
    if (!dir.exists()) return
    val script = File(repoRoot, "scripts/align_elf_16k.py")
    if (!script.exists()) return
    providers.exec {
        commandLine(pythonCmd() + listOf(script.absolutePath, dir.absolutePath))
        isIgnoreExitValue = true
    }.result.get()
}

android {
    namespace = "com.aspenini.quark"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.Aspenini.QuarkDownloader"
        minSdk = 26
        targetSdk = 35
        versionCode = 9
        versionName = "1.0.1"
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    signingConfigs {
        if (hasReleaseKeystore) {
            create("release") {
                storeFile = releaseStoreFile
                storePassword = releaseStorePassword
                keyAlias = releaseKeyAlias
                keyPassword = releaseKeyPassword
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
            if (hasReleaseKeystore) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }
    sourceSets.getByName("main") {
        jniLibs.srcDir("src/main/jniLibs")
    }
}

val cargoJni =
    tasks.register<Exec>("cargoJniLibs") {
        workingDir = repoRoot
        if (isWindows) {
            commandLine(
                "powershell.exe",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                File(repoRoot, "scripts/windows/build-android-jni.ps1").absolutePath,
            )
        } else {
            commandLine("bash", File(repoRoot, "scripts/unix/build-android-jni.sh").absolutePath)
        }
    }
tasks.named("preBuild").configure { dependsOn(cargoJni) }

val copyLicenseNotices =
    tasks.register<Copy>("copyLicenseNotices") {
        from(rootProject.file("LICENSE"), rootProject.file("APACHE-2.0"), rootProject.file("THIRD_PARTY_NOTICES.md"))
        into(generatedLicenseAssets.map { it.dir("licenses") })
    }
android.sourceSets.getByName("main").assets.srcDir(generatedLicenseAssets)
tasks.named("preBuild").configure { dependsOn(copyLicenseNotices) }

afterEvaluate {
    listOf(
        "mergeDebugNativeLibs",
        "mergeReleaseNativeLibs",
        "stripDebugDebugSymbols",
        "stripReleaseDebugSymbols",
    ).forEach { name ->
        tasks.findByName(name)?.doLast {
            outputs.files.forEach { file ->
                val dir = if (file.isDirectory) file else file.parentFile
                if (dir != null) alignSoTree(dir)
            }
        }
    }
}

dependencies {
    val ytdl = "0.18.1"
    implementation("io.github.junkfood02.youtubedl-android:library:$ytdl")
    implementation("io.github.junkfood02.youtubedl-android:ffmpeg:$ytdl")

    val composeBom = platform("androidx.compose:compose-bom:2024.10.01")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui-tooling-preview")
    debugImplementation("androidx.compose.ui:ui-tooling")

    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.activity:activity-ktx:1.9.3")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
}

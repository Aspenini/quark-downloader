import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

val repoRoot = rootProject.projectDir.parentFile
val isWindows = System.getProperty("os.name").orEmpty().startsWith("Windows")
val ksFile = rootProject.file("keystore.properties")
val ks = Properties()
val hasReleaseKeystore =
    ksFile.exists().also { exists ->
        if (exists) ksFile.inputStream().use { ks.load(it) }
    }

fun pythonCmd(): List<String> =
    if (isWindows) listOf("py", "-3") else listOf("python3")

fun alignSoTree(dir: File) {
    if (!dir.exists()) return
    val script = File(repoRoot, "scripts/align_elf_16k.py")
    if (!script.exists()) return
    exec {
        commandLine(pythonCmd() + listOf(script.absolutePath, dir.absolutePath))
        isIgnoreExitValue = true
    }
}

android {
    namespace = "com.aspenini.quark"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.Aspenini.QuarkDownloader"
        minSdk = 26
        targetSdk = 35
        versionCode = 7
        versionName = "0.7.0"
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    signingConfigs {
        if (hasReleaseKeystore) {
            create("release") {
                val store = File(ks.getProperty("storeFile"))
                storeFile = if (store.isAbsolute) store else rootProject.file(store)
                storePassword = ks.getProperty("storePassword")
                keyAlias = ks.getProperty("keyAlias")
                keyPassword = ks.getProperty("keyPassword")
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

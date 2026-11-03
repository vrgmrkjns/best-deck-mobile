//! What the app needs from the phone: storage, the device's name, a lit screen, a tap you can
//! feel, and handing an update to the system installer. The Windows build is for development.

use std::path::PathBuf;

/// The MIME type Android's installer opens.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
const APK_MIME: &str = "application/vnd.android.package-archive";

#[derive(Clone)]
pub struct Platform {
    #[cfg(target_os = "android")]
    app: winit::platform::android::activity::AndroidApp,
    /// The screen wake lock while the deck is on show.
    #[cfg(target_os = "android")]
    wake_lock: std::sync::Arc<std::sync::Mutex<Option<jni::objects::GlobalRef>>>,
    /// The last system bar insets and when they were read.
    #[cfg(target_os = "android")]
    insets: std::sync::Arc<std::sync::Mutex<(Option<std::time::Instant>, (f32, f32))>>,
}

#[cfg(target_os = "android")]
mod android {
    use jni::objects::{JObject, JString, JValue};
    use jni::{JNIEnv, JavaVM};

    use super::*;

    /// Android 10: MediaStore Downloads, used to hand the update to the installer.
    const MIN_SDK_FOR_UPDATES: i32 = 29;
    const FLAG_GRANT_READ_URI_PERMISSION: i32 = 0x0000_0001;
    const FLAG_ACTIVITY_NEW_TASK: i32 = 0x1000_0000;
    /// `VibrationEffect.EFFECT_TICK`.
    const EFFECT_TICK: i32 = 2;

    impl Platform {
        pub fn new(app: winit::platform::android::activity::AndroidApp) -> Self {
            Platform { app, wake_lock: Default::default(), insets: Default::default() }
        }

        /// Runs `work` with the JVM attached to this thread and the activity at hand.
        /// A Java exception is cleared and turned into an error.
        fn java<T>(&self, work: impl FnOnce(&mut JNIEnv, &JObject) -> jni::errors::Result<T>) -> Result<T, String> {
            let vm = unsafe { JavaVM::from_raw(self.app.vm_as_ptr().cast()) }.map_err(|e| e.to_string())?;
            let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
            let activity = unsafe { JObject::from_raw(self.app.activity_as_ptr().cast()) };
            let result = work(&mut env, &activity);
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_describe();
                let _ = env.exception_clear();
            }
            result.map_err(|e| e.to_string())
        }

        pub fn data_dir(&self) -> PathBuf {
            self.app.internal_data_path().unwrap_or_else(|| PathBuf::from("/data/local/tmp/best-deck"))
        }

        pub fn device_name(&self) -> String {
            self.java(|env, _| {
                let model = env.get_static_field("android/os/Build", "MODEL", "Ljava/lang/String;")?.l()?;
                let model: String = env.get_string(&JString::from(model))?.into();
                Ok(model)
            })
            .ok()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "Android telefon".into())
        }

        /// Pixels the status and navigation bars cover at the top and bottom of the window.
        /// Pixels the status and navigation bars cover at the top and bottom of the window.
        /// Android 15 draws every app edge to edge, so these come from the window's insets;
        /// they are read at most once a second.
        pub fn insets(&self) -> (f32, f32) {
            let mut cached = self.insets.lock().unwrap();
            if cached.0.is_some_and(|at: std::time::Instant| at.elapsed() < std::time::Duration::from_secs(1)) {
                return cached.1;
            }
            let read = self.java(|env, activity| {
                let window = env.call_method(activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
                let decor = env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;
                let insets = env.call_method(&decor, "getRootWindowInsets", "()Landroid/view/WindowInsets;", &[])?.l()?;
                if insets.is_null() {
                    return Ok((0.0, 0.0));
                }
                if Self::sdk(env)? >= 30 {
                    let types = env.call_static_method("android/view/WindowInsets$Type", "systemBars", "()I", &[])?.i()?;
                    let bars = env.call_method(&insets, "getInsets", "(I)Landroid/graphics/Insets;", &[JValue::Int(types)])?.l()?;
                    let top = env.get_field(&bars, "top", "I")?.i()?;
                    let bottom = env.get_field(&bars, "bottom", "I")?.i()?;
                    Ok((top as f32, bottom as f32))
                } else {
                    let top = env.call_method(&insets, "getSystemWindowInsetTop", "()I", &[])?.i()?;
                    let bottom = env.call_method(&insets, "getSystemWindowInsetBottom", "()I", &[])?.i()?;
                    Ok((top as f32, bottom as f32))
                }
            });
            *cached = (Some(std::time::Instant::now()), read.unwrap_or(cached.1));
            cached.1
        }

        /// Holds a screen wake lock while `on`. Window flags would do the same, but setting them
        /// through the activity glue can stall the drawing thread, so this goes through PowerManager.
        pub fn keep_screen_on(&self, on: bool) {
            /// `PowerManager.SCREEN_BRIGHT_WAKE_LOCK`.
            const SCREEN_BRIGHT: i32 = 0x0000_000a;
            let (platform, lock) = (self.clone(), self.wake_lock.clone());
            std::thread::spawn(move || {
                let mut held = lock.lock().unwrap();
                let _ = platform.java(|env, activity| {
                    if on && held.is_none() {
                        let name = env.new_string("power")?;
                        let power = env
                            .call_method(activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[JValue::Object(&name)])?
                            .l()?;
                        let tag = env.new_string("best-deck:deck")?;
                        let wake = env
                            .call_method(&power, "newWakeLock", "(ILjava/lang/String;)Landroid/os/PowerManager$WakeLock;", &[JValue::Int(SCREEN_BRIGHT), JValue::Object(&tag)])?
                            .l()?;
                        env.call_method(&wake, "setReferenceCounted", "(Z)V", &[JValue::Bool(0)])?;
                        env.call_method(&wake, "acquire", "()V", &[])?;
                        *held = Some(env.new_global_ref(wake)?);
                    } else if !on {
                        if let Some(wake) = held.take() {
                            env.call_method(wake.as_obj(), "release", "()V", &[])?;
                        }
                    }
                    Ok(())
                });
            });
        }

        fn sdk(env: &mut JNIEnv) -> jni::errors::Result<i32> {
            env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?.i()
        }

        /// A short tick, like a key landing.
        pub fn tick(&self) {
            let platform = self.clone();
            std::thread::spawn(move || {
                let _ = platform.java(|env, activity| {
                    if Self::sdk(env)? < 29 {
                        return Ok(());
                    }
                    let name = env.new_string("vibrator")?;
                    let vibrator = env
                        .call_method(activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[JValue::Object(&name)])?
                        .l()?;
                    if vibrator.is_null() {
                        return Ok(());
                    }
                    let effect = env
                        .call_static_method("android/os/VibrationEffect", "createPredefined", "(I)Landroid/os/VibrationEffect;", &[JValue::Int(EFFECT_TICK)])?
                        .l()?;
                    env.call_method(&vibrator, "vibrate", "(Landroid/os/VibrationEffect;)V", &[JValue::Object(&effect)])?;
                    Ok(())
                });
            });
        }

        /// Copies the APK into Downloads and opens the system installer on it.
        pub fn install(&self, apk: &[u8], version_code: u32) -> Result<(), String> {
            enum Outcome {
                Opened,
                OldAndroid,
                NeedsPermission,
            }
            let outcome = self.java(|env, activity| {
                if Self::sdk(env)? < MIN_SDK_FOR_UPDATES {
                    return Ok(Outcome::OldAndroid);
                }
                let manager = env.call_method(activity, "getPackageManager", "()Landroid/content/pm/PackageManager;", &[])?.l()?;
                if !env.call_method(&manager, "canRequestPackageInstalls", "()Z", &[])?.z()? {
                    // Open the "install unknown apps" switch for this app.
                    let package = env.call_method(activity, "getPackageName", "()Ljava/lang/String;", &[])?.l()?;
                    let package: String = env.get_string(&JString::from(package))?.into();
                    let text = env.new_string(format!("package:{package}"))?;
                    let uri = env.call_static_method("android/net/Uri", "parse", "(Ljava/lang/String;)Landroid/net/Uri;", &[JValue::Object(&text)])?.l()?;
                    let action = env.new_string("android.settings.MANAGE_UNKNOWN_APP_SOURCES")?;
                    let intent = env.new_object("android/content/Intent", "(Ljava/lang/String;Landroid/net/Uri;)V", &[JValue::Object(&action), JValue::Object(&uri)])?;
                    env.call_method(&intent, "addFlags", "(I)Landroid/content/Intent;", &[JValue::Int(FLAG_ACTIVITY_NEW_TASK)])?;
                    env.call_method(activity, "startActivity", "(Landroid/content/Intent;)V", &[JValue::Object(&intent)])?;
                    return Ok(Outcome::NeedsPermission);
                }

                let resolver = env.call_method(activity, "getContentResolver", "()Landroid/content/ContentResolver;", &[])?.l()?;
                let downloads = env.get_static_field("android/provider/MediaStore$Downloads", "EXTERNAL_CONTENT_URI", "Landroid/net/Uri;")?.l()?;

                // Our earlier update files; the app may only remove its own.
                let selection = env.new_string("_display_name LIKE ?")?;
                let pattern = env.new_string("best-deck-%.apk")?;
                let args = env.new_object_array(1, "java/lang/String", &pattern)?;
                let _ = env.call_method(
                    &resolver,
                    "delete",
                    "(Landroid/net/Uri;Ljava/lang/String;[Ljava/lang/String;)I",
                    &[JValue::Object(&downloads), JValue::Object(&selection), JValue::Object(&args)],
                );
                if env.exception_check()? {
                    env.exception_clear()?;
                }

                let values = env.new_object("android/content/ContentValues", "()V", &[])?;
                for (key, value) in [("_display_name", format!("best-deck-{version_code}.apk")), ("mime_type", APK_MIME.into()), ("relative_path", "Download/".into())] {
                    let (key, value) = (env.new_string(key)?, env.new_string(value)?);
                    env.call_method(&values, "put", "(Ljava/lang/String;Ljava/lang/String;)V", &[JValue::Object(&key), JValue::Object(&value)])?;
                }
                let uri = env
                    .call_method(&resolver, "insert", "(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;", &[JValue::Object(&downloads), JValue::Object(&values)])?
                    .l()?;
                let out = env.call_method(&resolver, "openOutputStream", "(Landroid/net/Uri;)Ljava/io/OutputStream;", &[JValue::Object(&uri)])?.l()?;
                for chunk in apk.chunks(256 * 1024) {
                    let bytes = env.byte_array_from_slice(chunk)?;
                    env.call_method(&out, "write", "([B)V", &[JValue::Object(&bytes)])?;
                    env.delete_local_ref(bytes)?;
                }
                env.call_method(&out, "close", "()V", &[])?;

                let action = env.new_string("android.intent.action.VIEW")?;
                let intent = env.new_object("android/content/Intent", "(Ljava/lang/String;)V", &[JValue::Object(&action)])?;
                let mime = env.new_string(APK_MIME)?;
                env.call_method(&intent, "setDataAndType", "(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;", &[JValue::Object(&uri), JValue::Object(&mime)])?;
                env.call_method(&intent, "addFlags", "(I)Landroid/content/Intent;", &[JValue::Int(FLAG_GRANT_READ_URI_PERMISSION | FLAG_ACTIVITY_NEW_TASK)])?;
                env.call_method(activity, "startActivity", "(Landroid/content/Intent;)V", &[JValue::Object(&intent)])?;
                Ok(Outcome::Opened)
            })?;
            match outcome {
                Outcome::Opened => Ok(()),
                Outcome::NeedsPermission => Err("Engedélyezd a telepítést ebből az alkalmazásból, aztán próbáld újra.".into()),
                Outcome::OldAndroid => Err("Ezen az Android-verzión a frissítést kézzel kell telepíteni.".into()),
            }
        }
    }
}

#[cfg(not(target_os = "android"))]
impl Platform {
    pub fn new() -> Self {
        Platform {}
    }

    /// `BEST_DECK_PHONE_DATA`, or a folder under `%APPDATA%` kept apart from the PC host's.
    pub fn data_dir(&self) -> PathBuf {
        if let Some(dir) = std::env::var_os("BEST_DECK_PHONE_DATA") {
            return PathBuf::from(dir);
        }
        let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        base.join("best-deck-phone-dev").join("files")
    }

    pub fn device_name(&self) -> String {
        "Fejlesztői ablak".into()
    }

    pub fn insets(&self) -> (f32, f32) {
        (0.0, 0.0)
    }

    pub fn keep_screen_on(&self, _on: bool) {}

    pub fn tick(&self) {}

    pub fn install(&self, _apk: &[u8], _version_code: u32) -> Result<(), String> {
        Err("Frissíteni csak a telefonon lehet.".into())
    }
}

//! Hearth installer plugin. Hands a staged APK to the Android
//! package installer. Desktop builds refuse: nothing to install.

use tauri::{plugin::TauriPlugin, Manager, Runtime};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "app.hearth.installer";

/// Access to the installer from a running app.
pub struct Installer<R: Runtime> {
    #[cfg(target_os = "android")]
    mobile_plugin_handle: PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _marker: std::marker::PhantomData<fn() -> R>,
}

impl<R: Runtime> Installer<R> {
    /// Install the APK at `path` through a PackageInstaller session.
    /// Android shows the system confirm dialog. Refuses off Android.
    pub fn install(&self, path: &str) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.mobile_plugin_handle
                .run_mobile_plugin("install", serde_json::json!({ "path": path }))
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = path;
            Err("install needs the Android app".to_string())
        }
    }
}

/// Extension trait. Reach the installer from any app handle.
pub trait InstallerExt<R: Runtime> {
    /// The installer API.
    fn installer(&self) -> &Installer<R>;
}

impl<R: Runtime, T: Manager<R>> InstallerExt<R> for T {
    fn installer(&self) -> &Installer<R> {
        self.state::<Installer<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("hearth-installer")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "InstallerPlugin")?;
                app.manage(Installer {
                    mobile_plugin_handle: handle,
                });
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = api;
                app.manage(Installer {
                    _marker: std::marker::PhantomData::<fn() -> R>,
                });
            }
            Ok(())
        })
        .build()
}

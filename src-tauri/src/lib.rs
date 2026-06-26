use std::sync::{Arc, Mutex};
use tauri::{Emitter, Listener, Manager};

// AUMID (AppUserModelID) de l'app. DOIT rester identique à `identifier` dans
// tauri.conf.json ET à la propriété System.AppUserModel.ID du raccourci Menu Démarrer
// créé par l'installeur — sinon Windows ignore silencieusement les Toast notifications.
#[cfg(target_os = "windows")]
const APP_AUMID: &str = "fr.fastpanel.fasttask";

// Notifie Windows Shell que les associations/icônes ont changé.
// Équivalent à ce que font les installeurs signés automatiquement — force
// le rafraîchissement du cache d'icônes sans redémarrage de l'explorateur.
#[cfg(target_os = "windows")]
fn refresh_shell_icons() {
    use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
    unsafe {
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }
}

// Associe explicitement le PROCESSUS courant à l'AUMID de l'app. Sans cela, le toast
// peut être émis sous une autre identité (ex. PowerShell) ou avalé silencieusement par
// le Centre de notifications. Méthode recommandée par Microsoft.
#[cfg(target_os = "windows")]
fn set_app_user_model_id() {
    use windows::core::w;
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
    // w! exige un littéral ; il doit correspondre à APP_AUMID.
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(w!("fr.fastpanel.fasttask"));
    }
}

// Enregistre les métadonnées d'affichage de l'AUMID (nom affiché du toast) dans le
// registre. Complète — sans le remplacer — le raccourci Menu Démarrer posé par l'installeur.
#[cfg(target_os = "windows")]
fn register_windows_toast_aumid() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) =
        hkcu.create_subkey(format!("SOFTWARE\\Classes\\AppUserModelId\\{APP_AUMID}"))
    {
        let _ = key.set_value("DisplayName", &"FastTask");
    }
}

#[cfg(desktop)]
use tauri_plugin_updater::UpdaterExt;

#[cfg(desktop)]
type Pending = Arc<Mutex<Option<(tauri_plugin_updater::Update, Vec<u8>)>>>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "windows")]
    {
        refresh_shell_icons();
        set_app_user_model_id();
        register_windows_toast_aumid();
    }

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            #[cfg(desktop)]
            setup_tray(app)?;
            // Applique explicitement l'icône .ico sur la fenêtre principale (barre des tâches Windows)
            #[cfg(target_os = "windows")]
            if let Some(win) = app.get_webview_window("main") {
                if let Ok(icon) = app.default_window_icon().cloned().ok_or(()) {
                    let _ = win.set_icon(icon);
                }
            }
            #[cfg(desktop)]
            {
                let pending: Pending = Arc::new(Mutex::new(None));
                setup_updater(app, pending);
            }
            Ok(())
        });

    #[cfg(desktop)]
    {
        builder = builder.on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        });
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Vérifie les mises à jour et gère le cycle download → install via événements Tauri.
#[cfg(desktop)]
fn setup_updater(app: &mut tauri::App, pending: Pending) {
    let app_handle = app.handle().clone();

    // Vérifie les mises à jour au démarrage (délai pour laisser le webview charger)
    {
        let app_check = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            if let Ok(updater) = app_check.updater() {
                if let Ok(Some(update)) = updater.check().await {
                    let _ = app_check.emit(
                        "fasttask-update-available",
                        serde_json::json!({
                            "version": update.version,
                            "currentVersion": update.current_version,
                        }),
                    );
                }
            }
        });
    }

    // Déclenché par le frontend quand l'utilisateur clique "Mettre à jour"
    {
        let app_dl = app_handle.clone();
        let pending_dl = pending.clone();
        app.listen("fasttask-start-download", move |_| {
            let app_dl = app_dl.clone();
            let pending_dl = pending_dl.clone();
            tauri::async_runtime::spawn(async move {
                let Ok(updater) = app_dl.updater() else { return };
                let Ok(Some(update)) = updater.check().await else { return };

                let app_progress = app_dl.clone();
                let mut downloaded = 0u64;

                let bytes_result = update
                    .download(
                        move |chunk, total| {
                            downloaded += chunk as u64;
                            let percent = total
                                .map(|t| if t > 0 { (downloaded * 100 / t) as u8 } else { 0 })
                                .unwrap_or(0);
                            let _ = app_progress.emit(
                                "fasttask-update-progress",
                                serde_json::json!({ "percent": percent }),
                            );
                        },
                        || {},
                    )
                    .await;

                match bytes_result {
                    Ok(bytes) => {
                        *pending_dl.lock().unwrap() = Some((update, bytes));
                        let _ = app_dl.emit("fasttask-update-ready", ());
                    }
                    Err(e) => {
                        let _ = app_dl.emit("fasttask-update-error", e.to_string());
                    }
                }
            });
        });
    }

    // Déclenché par le frontend quand l'utilisateur clique "Redémarrer maintenant"
    {
        let app_inst = app_handle;
        app.listen("fasttask-install-update", move |_| {
            let maybe_pending = {
                let mut lock = pending.lock().unwrap();
                lock.take()
            };
            if let Some((update, bytes)) = maybe_pending {
                if let Err(e) = update.install(bytes) {
                    let _ = app_inst.emit("fasttask-update-error", e.to_string());
                }
            }
        });
    }
}

#[cfg(desktop)]
fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    };

    let show = MenuItem::with_id(app, "show", "Afficher FastTask", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("FastTask")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

#![cfg_attr(windows, windows_subsystem = "windows")]
fn main() {
    #[cfg(windows)]
    {
        let args = std::env::args_os().collect::<Vec<_>>();
        if args.get(1).is_some_and(|v| v == "--migrate-legacy-install") {
            let result = if args.len() == 3 {
                std::env::current_exe()
                    .map_err(|e| e.to_string())
                    .and_then(|exe| {
                        let root = exe.parent().ok_or("Installationsordner fehlt")?;
                        diktator::bundle::Bundle::installed(&exe)?
                            .ok_or("Neue Installation ist unvollständig")?;
                        // Migrate user data before retiring the old program files.
                        diktator::migration::load_data(
                            &diktator::platform::data_dir(),
                            &diktator::platform::old_dir(),
                        )?;
                        diktator::migration::retire_install(std::path::Path::new(&args[2]), root)
                    })
            } else {
                Err("Ungültige Migration".into())
            };
            if let Err(e) = result {
                eprintln!("{e}");
                std::process::exit(12);
            }
            return;
        }
        if args.get(1).is_some_and(|v| v == "--asr-worker") {
            let result = if args.len() == 5 {
                args[3]
                    .to_string_lossy()
                    .parse::<usize>()
                    .map_err(|e| e.to_string())
                    .and_then(|threads| {
                        args[4]
                            .to_string_lossy()
                            .parse::<u32>()
                            .map_err(|e| e.to_string())
                            .and_then(|pid| {
                                diktator::worker::run(std::path::Path::new(&args[2]), threads, pid)
                            })
                    })
            } else {
                Err("Ungültige ASR-Argumente.".into())
            };
            if result.is_err() {
                std::process::exit(11);
            }
            return;
        }
        if args
            .get(1)
            .is_some_and(|v| v == "--quit-existing" || v == "--verify-install")
        {
            let result = if args[1] == "--quit-existing" {
                diktator::ui::stop_existing()
            } else {
                std::env::current_exe()
                    .map_err(|e| e.to_string())
                    .and_then(|exe| {
                        diktator::bundle::Bundle::installed(&exe)?
                            .ok_or("Installierte Programmdateien fehlen.".into())
                            .map(|_| ())
                    })
            };
            if let Err(e) = result {
                eprintln!("{e}");
                std::process::exit(10);
            }
            return;
        }
        if args.get(1).is_some_and(|v| v == "--apply-update") {
            let result = if args.len() == 4 {
                args[3]
                    .to_string_lossy()
                    .parse::<u32>()
                    .map_err(|e| e.to_string())
                    .and_then(|pid| diktator::update::helper(std::path::Path::new(&args[2]), pid))
            } else {
                Err("Ungültige Update-Argumente.".into())
            };
            if let Err(e) = result {
                diktator::platform::error_box(&e)
            }
            return;
        }
        if let Err(e) = diktator::ui::run() {
            diktator::platform::error_box(&e);
        }
    }
    #[cfg(not(windows))]
    {
        eprintln!("Diktator ist eine Windows-Anwendung. cargo test führt die plattformunabhängigen Tests aus.");
    }
}

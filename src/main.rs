#![cfg_attr(windows, windows_subsystem = "windows")]
fn main() {
    #[cfg(windows)]
    {
        let args = std::env::args_os().collect::<Vec<_>>();
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
        eprintln!("Silbenlot ist eine Windows-Anwendung. cargo test führt die plattformunabhängigen Tests aus.");
    }
}

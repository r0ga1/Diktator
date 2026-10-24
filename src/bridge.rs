//! Private, bounded JSON-lines bridge. Production uses anonymous child pipes, no TCP listener.
use crate::{
    app::{App, Control},
    platform,
    store::{self, Replacement, Settings},
};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};

pub fn call(a: &Arc<App>, method: &str, v: Value) -> Result<Value, String> {
    let owner = match a.main_window.load(Ordering::Relaxed) {
        0 => a.native_window.load(Ordering::Relaxed),
        n => n,
    };
    match method {
        "state" => {
            let s = a.snapshot();
            let x = a.audio.lock().unwrap();
            let settings = a.settings();
            let stamp = {
                let d = a.data.lock().unwrap();
                format!(
                    "{}:{}",
                    d.history.len(),
                    d.history.first().map_or("", |e| e.id.as_str())
                )
            };
            Ok(
                json!({"version":crate::VERSION,"arch":std::env::consts::ARCH,"phase":format!("{:?}",s.phase),"ready":s.ready,"modelLoaded":s.model_loaded,"activeOutputMode":s.output_mode,"inputPaused":a.input_paused.load(Ordering::SeqCst),"status":s.status,"error":s.error,"warning":s.warning,"text":if v["includeText"].as_bool().unwrap_or(false){s.text}else{String::new()},"update":s.update,"updateRepository":crate::update::repository(),"updateChannel":crate::update_trust::config().channel,"progress":s.progress,"recordSeconds":s.started.map_or(0.,|i|i.elapsed().as_secs_f64()),"level":x.level,"peak":x.peak,"nativeRate":x.native_rate,"channels":x.channels,"device":x.device,"inputError":x.error,"captureShortcut":a.capture_shortcut.load(Ordering::Relaxed),"shortcut":store::shortcut_label(settings.shortcut),"cancelShortcut":store::shortcut_label(settings.cancel_shortcut),"historyStamp":stamp,"settings":settings}),
            )
        }
        "previewSound" => {
            let start = v["start"].as_bool().unwrap_or(true);
            let settings = a.settings();
            // Preview runs off the pipe thread, and never activates microphone/history.
            std::thread::spawn(move || {
                platform::play_sound(
                    start,
                    true,
                    if start {
                        settings.start_volume
                    } else {
                        settings.stop_volume
                    },
                )
            });
            Ok(json!(true))
        }
        "dashboard" => {
            let d = a.data.lock().unwrap();
            let m = store::metrics(&d.history);
            Ok(
                json!({"sessions":m.sessions,"words":m.words,"seconds":m.seconds,"saved":m.saved,"wpm":m.wpm,"keystrokes":m.keystrokes,"recent":d.history.iter().take(3).collect::<Vec<_>>()}),
            )
        }
        "history" => {
            let q = v["q"].as_str().unwrap_or("").to_lowercase();
            let offset = v["offset"].as_u64().unwrap_or(0).min(100000) as usize;
            let d = a.data.lock().unwrap();
            let items = d
                .history
                .iter()
                .filter(|e| {
                    q.is_empty()
                        || e.text.to_lowercase().contains(&q)
                        || e.name.to_lowercase().contains(&q)
                })
                .collect::<Vec<_>>();
            Ok(
                json!({"total":items.len(),"entries":items.into_iter().skip(offset).take(40).collect::<Vec<_>>()}),
            )
        }
        "entry" => {
            let d = a.data.lock().unwrap();
            Ok(json!(d
                .history
                .iter()
                .find(|e| e.id == v["id"].as_str().unwrap_or(""))))
        }
        "editEntry" => {
            let mut d = a.data.lock().unwrap();
            let old = d.clone();
            let e = d
                .history
                .iter_mut()
                .find(|e| e.id == v["id"].as_str().unwrap_or(""))
                .ok_or("Eintrag nicht gefunden")?;
            let text = v["text"].as_str().ok_or("Text fehlt")?;
            if text.len() > 4_000_000 {
                return Err("Text zu lang".into());
            }
            e.text = text.into();
            if let Some(n) = v["name"].as_str() {
                e.name = n.chars().take(160).collect();
            }
            if let Err(e) = d.save(&a.dir) {
                *d = old;
                return Err(e);
            }
            Ok(json!(true))
        }
        "deleteEntry" => {
            let id = v["id"].as_str().unwrap_or("");
            let mut d = a.data.lock().unwrap();
            let old = d.clone();
            d.history.retain(|e| e.id != id);
            if let Err(e) = d.save(&a.dir) {
                *d = old;
                return Err(e);
            }
            if store::safe_id(id) {
                let _ = std::fs::remove_file(a.dir.join(format!("{id}.wav")));
            }
            Ok(json!(true))
        }
        "copy" => {
            platform::copy_text(owner, v.as_str().unwrap_or(""))?;
            Ok(json!(true))
        }
        "exportEntry" => {
            let e = a
                .data
                .lock()
                .unwrap()
                .history
                .iter()
                .find(|e| e.id == v["id"].as_str().unwrap_or(""))
                .cloned()
                .ok_or("Eintrag nicht gefunden")?;
            if let Some(p) = platform::file_dialog(owner, true) {
                let bytes = if p
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("json"))
                {
                    serde_json::to_vec_pretty(&e).map_err(|e| e.to_string())?
                } else {
                    e.text.into_bytes()
                };
                std::fs::write(p, bytes).map_err(|e| e.to_string())?;
                Ok(json!(true))
            } else {
                Ok(json!(false))
            }
        }
        "playEntry" => {
            let d = a.data.lock().unwrap();
            let e = d
                .history
                .iter()
                .find(|e| e.id == v["id"].as_str().unwrap_or(""))
                .ok_or("Eintrag nicht gefunden")?;
            if !e.audio || !store::safe_id(&e.id) {
                return Err("Kein Audio aufbewahrt".into());
            }
            platform::open_resource(
                owner,
                &a.dir.join(format!("{}.wav", e.id)).to_string_lossy(),
            )?;
            Ok(json!(true))
        }
        "devices" => Ok(json!(platform::microphone::devices())),
        "settings" => {
            let s: Settings = serde_json::from_value(v).map_err(|e| e.to_string())?;
            a.save_settings(s)?;
            Ok(json!(true))
        }
        "dictionary" => {
            if v.is_null() {
                return Ok(json!(a.data.lock().unwrap().dictionary));
            }
            let rules: Vec<Replacement> = serde_json::from_value(v).map_err(|e| e.to_string())?;
            if rules.len() > 500
                || rules
                    .iter()
                    .any(|r| r.from.len() > 500 || r.to.len() > 2000 || r.from.trim().is_empty())
            {
                return Err("Maximal 500 Regeln, mit nicht leerem Ausgangswort".into());
            }
            let mut d = a.data.lock().unwrap();
            let old = std::mem::replace(&mut d.dictionary, rules);
            if let Err(e) = d.save(&a.dir) {
                d.dictionary = old;
                return Err(e);
            }
            Ok(json!(true))
        }
        "micTest" => {
            a.control.send(Control::Test).map_err(|e| e.to_string())?;
            Ok(json!(true))
        }
        "stop" => {
            if a.recording.load(Ordering::SeqCst) {
                a.control.send(Control::Toggle).map_err(|e| e.to_string())?;
            }
            Ok(json!(true))
        }
        "cancel" => {
            a.control.send(Control::Cancel).map_err(|e| e.to_string())?;
            Ok(json!(true))
        }
        "shortcutCapture" => {
            a.capture_shortcut.store(
                match v.as_str() {
                    Some("record") => 1,
                    Some("cancel") => 2,
                    _ => 0,
                },
                Ordering::SeqCst,
            );
            Ok(json!(true))
        }
        "attachWindow" => {
            a.main_window
                .store(v.as_i64().unwrap_or(0) as isize, Ordering::SeqCst);
            Ok(json!(true))
        }
        "import" => {
            if let Some(p) = platform::file_dialog(owner, false) {
                a.queue_file(p)?;
                Ok(json!(true))
            } else {
                Ok(json!(false))
            }
        }
        "importPath" if std::env::var_os("DIKTATOR_DEV_ROOT").is_some() => {
            a.queue_file(PathBuf::from(v.as_str().ok_or("Pfad fehlt")?))?;
            Ok(json!(true))
        }
        "legal" => {
            let file = match v.as_str() {
                Some("gpl") => "LICENSE",
                Some("thirdParty") => "THIRD_PARTY.md",
                Some("permission") => "ADDITIONAL_PERMISSION.md",
                Some("privacy") => "PRIVACY.md",
                Some("microsoft") => "licenses/WebView2-FixedVersion-Microsoft.txt",
                _ => return Err("Unbekannter Lizenztext.".into()),
            };
            let text = std::fs::read_to_string(a.root.join(file)).map_err(|e| e.to_string())?;
            Ok(json!(text))
        }
        "projectPage" => {
            platform::open_resource(
                owner,
                &format!("https://github.com/{}", crate::update::repository()),
            )?;
            Ok(json!(true))
        }
        "permissions" => {
            platform::open_resource(owner, "ms-settings:privacy-microphone")?;
            Ok(json!(true))
        }
        "dataFolder" => {
            platform::open_resource(owner, &a.dir.to_string_lossy())?;
            Ok(json!(true))
        }
        "checkUpdate" => {
            crate::update::check_now(a.clone());
            Ok(json!(true))
        }
        "releaseModel" => {
            a.release_idle_model();
            Ok(json!(true))
        }
        "quit" => {
            crate::ui::request_quit();
            Ok(json!(true))
        }
        _ => Err("Unbekannter App-Befehl".into()),
    }
}
/// Each request is handled once, in order; no unsolicited transcript logging.
pub fn serve(a: Arc<App>, input: impl Read, mut output: impl Write) {
    let mut reader = BufReader::new(input);
    loop {
        let mut bytes = Vec::new();
        match reader
            .by_ref()
            .take(8 * 1024 * 1024 + 1)
            .read_until(b'\n', &mut bytes)
        {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        if bytes.len() > 8 * 1024 * 1024 {
            break;
        }
        let result = (|| {
            let v: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let id = v["id"].clone();
            let r = call(&a, v["method"].as_str().unwrap_or(""), v["params"].clone());
            Ok::<_, String>(match r {
                Ok(data) => json!({"id":id,"result":data}),
                Err(e) => json!({"id":id,"error":e}),
            })
        })();
        let value = result.unwrap_or_else(|e| json!({"id":null,"error":e}));
        if serde_json::to_writer(&mut output, &value).is_err()
            || output.write_all(b"\n").is_err()
            || output.flush().is_err()
        {
            break;
        }
    }
}

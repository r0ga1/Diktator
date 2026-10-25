use diktator::{
    asr::Recognizer,
    audio, edit,
    store::{Entry, Replacement, Store},
};
use std::{path::PathBuf, sync::atomic::AtomicBool, time::Instant};
#[test]
fn persistent_real_parakeet_and_live_edits() {
    let Some(root) = std::env::var_os("DIKTATOR_TEST_ROOT").map(PathBuf::from) else {
        eprintln!("Real model test disabled; set DIKTATOR_TEST_ROOT");
        return;
    };
    let runtime = std::env::var_os("DIKTATOR_TEST_RUNTIME")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("runtime"));
    let model = root.join("model");
    let start = Instant::now();
    let mut recognizer = Recognizer::load(&runtime, &model, 4).unwrap();
    println!("Persistent load {:.2}s", start.elapsed().as_secs_f64());
    for (lang, expect) in [("de", "Wurst"), ("en", "country"), ("fr", "pays")] {
        let pcm = audio::read_file(&model.join("test_wavs").join(format!("{lang}.wav"))).unwrap();
        let t = Instant::now();
        let text = recognizer.recognize(&pcm).unwrap();
        println!("{lang}: {text} ({:.3}s warm)", t.elapsed().as_secs_f64());
        assert!(text.contains(expect), "{text}");
        let mut typed = String::new();
        for n in [pcm.len() / 2, pcm.len()] {
            let partial = recognizer.recognize(&pcm[..n]).unwrap();
            let p = edit::patch(&typed, &partial);
            use unicode_segmentation::UnicodeSegmentation;
            let old = typed.graphemes(true).collect::<Vec<_>>();
            typed = old[..old.len() - p.backspaces].concat() + &p.insert;
        }
        assert_eq!(typed, text);
        let final_text = recognizer
            .recognize_full(&pcm, &AtomicBool::new(false))
            .unwrap();
        assert_eq!(typed, final_text);
    }
}
#[test]
fn migration_excludes_removed_features_and_preserves_history() {
    let root = std::env::temp_dir().join(format!("diktator-test-{}", diktator::store::new_id()));
    let old = root.join("old");
    let new = root.join("new");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::create_dir(&new).unwrap();
    std::fs::write(old.join("data.json"),r#"{"settings":{"threads":2,"enhancement":"local","modelId":"whisper-tiny","githubRepo":"old/voiceink"},"history":[{"id":"keep","text":"Hallo","original":"hallo","duration":2}],"dictionary":[{"from":"cat","to":"dog"}]}"#).unwrap();
    let data = Store::load(&new, Some(&old)).unwrap();
    assert_eq!(data.history[0].text, "Hallo");
    assert_eq!(data.settings.threads, 2);
    assert!(serde_json::to_value(&data.settings)
        .unwrap()
        .get("githubRepo")
        .is_none());
    let json = std::fs::read_to_string(new.join("data.json")).unwrap();
    for removed in [
        "enhancement",
        "modelId",
        "whisper",
        "ollama",
        "showPromotion",
    ] {
        assert!(!json.contains(removed))
    }
    let mut store = Store::load(&new, None).unwrap();
    store.history.push(Entry {
        text: edit::dictionary(
            "cat",
            &[Replacement {
                from: "cat".into(),
                to: "dog".into(),
            }],
        ),
        ..Default::default()
    });
    store.save(&new).unwrap();
    assert_eq!(Store::load(&new, None).unwrap().history[1].text, "dog");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn idle_and_tray_defaults_migrate_without_overwriting_later_choices() {
    let fresh = diktator::store::Settings::default();
    assert!(fresh.close_to_background && fresh.tray_setup);
    assert!(!fresh.keep_model_warm);
    let legacy: diktator::store::Settings =
        serde_json::from_str(r#"{"closeToBackground":false}"#).unwrap();
    assert!(!legacy.tray_setup);
    assert!(!legacy.keep_model_warm);
    let stored: diktator::store::Settings =
        serde_json::from_str(r#"{"closeToBackground":false,"traySetup":true}"#).unwrap();
    assert!(stored.tray_setup);
    assert!(!stored.close_to_background);
}

#[test]
fn independent_sound_volumes_migrate_validate_and_roundtrip() {
    use diktator::store::Settings;
    let mut s: Settings = serde_json::from_str(r#"{"sounds":true}"#).unwrap();
    assert_eq!((s.start_volume, s.stop_volume), (35, 35));
    s.start_volume = 0;
    s.stop_volume = 100;
    assert!(s.validate().is_ok());
    let copy: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!((copy.start_volume, copy.stop_volume), (0, 100));
    s.stop_volume = 101;
    assert!(s.validate().is_err());
}

#[test]
fn real_speech_gate_rejects_noise_but_keeps_quiet_german_and_english() {
    let Some(root) = std::env::var_os("DIKTATOR_TEST_ROOT").map(PathBuf::from) else {
        return;
    };
    let runtime = std::env::var_os("DIKTATOR_TEST_RUNTIME")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("runtime"));
    let mut r = Recognizer::load(&runtime, &root.join("model"), 4).unwrap();
    let n = 16000 * 4;
    let mut seed = 1234567u32;
    let noise = (0..n)
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            ((seed >> 8) as f32 / 16777216. - 0.5) * 0.04
        })
        .collect::<Vec<_>>();
    let hum = (0..n)
        .map(|i| (i as f32 * std::f32::consts::TAU * 60. / 16000.).sin() * 0.02)
        .collect::<Vec<_>>();
    let mut clicks = vec![0.; n];
    for i in (0..n).step_by(5300) {
        for x in &mut clicks[i..(i + 30).min(n)] {
            *x = 0.4;
        }
    }
    for (label, samples) in [
        ("silence", vec![0.; n]),
        ("fan-like noise", noise.clone()),
        ("mains hum", hum),
        ("clicks", clicks),
    ] {
        let text = r.recognize(&samples).unwrap();
        println!("speech gate {label}: {text:?}");
        assert!(text.is_empty(), "Noise generated text: {label}: {text}");
    }
    let de = audio::read_file(&root.join("model/test_wavs/de.wav")).unwrap();
    let quiet = de.iter().map(|v| v * 0.08).collect::<Vec<_>>();
    let text = r.recognize(&quiet).unwrap();
    println!("quiet German: {text}");
    assert!(text.contains("Wurst"), "{text}");
    let mut padded = noise.clone();
    padded.extend_from_slice(&de);
    padded.extend_from_slice(&noise);
    let text = r.recognize(&padded).unwrap();
    println!("German with surrounding noise: {text}");
    assert!(text.contains("Wurst"), "{text}");
    let en = audio::read_file(&root.join("model/test_wavs/en.wav")).unwrap();
    let mut mixed = de;
    mixed.extend(vec![0.; 8000]);
    mixed.extend_from_slice(&en);
    let text = r.recognize(&mixed).unwrap();
    println!("German + English: {text}");
    assert!(
        text.contains("Wurst") && text.to_lowercase().contains("country"),
        "{text}"
    );
}

#[test]
fn output_modes_migrate_without_disabling_live_and_roundtrip() {
    use diktator::store::{OutputMode, Settings};
    let old: Settings = serde_json::from_str(r#"{"copyFinal":false,"showOverlay":false}"#).unwrap();
    assert_eq!(old.output_mode, OutputMode::Live);
    assert!(!old.copy_final && !old.show_overlay);
    for (name, mode) in [
        ("live", OutputMode::Live),
        ("paste", OutputMode::Paste),
        ("clipboard", OutputMode::Clipboard),
    ] {
        let s: Settings = serde_json::from_value(serde_json::json!({"outputMode":name})).unwrap();
        assert_eq!(s.output_mode, mode);
        assert!(s.validate().is_ok());
        let restored: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(restored.output_mode, mode);
    }
    assert!(serde_json::from_str::<Settings>(r#"{"outputMode":"unsupported"}"#).is_err());
    assert!(!OutputMode::Clipboard.inserts());
    assert!(OutputMode::Paste.inserts() && OutputMode::Live.inserts());
}

#[test]
fn real_quiet_sentence_edges_and_long_german_context_are_retained() {
    let Some(quality) = std::env::var_os("DIKTATOR_QUALITY_ROOT").map(PathBuf::from) else {
        return;
    };
    let root = PathBuf::from(std::env::var_os("DIKTATOR_TEST_ROOT").unwrap());
    let runtime = std::env::var_os("DIKTATOR_TEST_RUNTIME")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("runtime"));
    let mut r = Recognizer::load(&runtime, &root.join("model"), 4).unwrap();
    for (file, required) in [
        (
            "de_de/10376367261184486277.wav",
            vec!["thermisches Verhalten", "Northern Arizona University"],
        ),
        (
            "en_us/10233995782544396174.wav",
            vec!["Many people", "dinosaurs", "can fly"],
        ),
    ] {
        let pcm = audio::read_file(&quality.join(file)).unwrap();
        let text = r.recognize_full(&pcm, &AtomicBool::new(false)).unwrap();
        println!("Retained sentence edges: {text}");
        for word in required {
            assert!(text.contains(word), "Lost speech: {word}: {text}");
        }
    }
}

#[test]
fn shortcuts_preserve_modifier_sides_and_legacy_bindings() {
    use diktator::store::{physical_key, shortcut_label, shortcut_matches as matches, Shortcut};
    let right = Shortcut {
        key: 163,
        modifiers: 0,
    };
    assert!(matches(right, 163, 32));
    assert!(!matches(right, 162, 16));
    assert!(!matches(right, 163, 32 | 16));
    assert_eq!(shortcut_label(right), "Strg rechts");
    let right_space = Shortcut {
        key: 32,
        modifiers: 32,
    };
    assert!(matches(right_space, 32, 32));
    assert!(!matches(right_space, 32, 16));
    assert!(!matches(right_space, 32, 32 | 256));
    let left_shift_right_ctrl = Shortcut {
        key: 65,
        modifiers: 32 | 256,
    };
    assert!(matches(left_shift_right_ctrl, 65, 32 | 256));
    assert!(!matches(left_shift_right_ctrl, 65, 16 | 256));
    assert!(!matches(left_shift_right_ctrl, 65, 32 | 512));
    let legacy = Shortcut {
        key: 32,
        modifiers: 3,
    };
    assert!(matches(legacy, 32, 16 | 64));
    assert!(matches(legacy, 32, 32 | 64));
    assert!(!matches(legacy, 32, 32));
    assert_eq!(physical_key(17, 0x1d, true), 163);
    assert_eq!(physical_key(17, 0x1d, false), 162);
    assert_eq!(physical_key(16, 0x36, false), 161);
    assert_eq!(physical_key(18, 0x38, true), 165);
    let settings = diktator::store::Settings {
        shortcut: right_space,
        ..Default::default()
    };
    assert!(settings.validate().is_ok());
    let copy: diktator::store::Settings =
        serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
    assert_eq!(copy.shortcut, right_space);
}

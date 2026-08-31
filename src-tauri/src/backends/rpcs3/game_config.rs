//! Per-game emulator settings.
//!
//! RPCS3 keeps these in `config/custom_configs/config_<TITLE_ID>.yml`, and it
//! merges what it finds there over its defaults. That was checked rather than
//! assumed: a three-line file naming only a renderer changed the renderer and
//! left everything else alone.
//!
//! So Omoio writes only what the user actually chose. Nothing is invented and
//! nothing is defaulted on their behalf: guessing at these makes games
//! slower, and only a setting someone asked for has a source behind it.

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tauri::AppHandle;

/// One knob, as the interface should present it. Every option list here comes
/// from RPCS3's own system_config_types.cpp, so the values it writes are ones
/// the emulator actually accepts.
#[derive(Serialize)]
pub struct Option_ {
    pub key: &'static str,
    pub section: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub kind: &'static str,
    pub choices: &'static [&'static str],
    pub min: i64,
    pub max: i64,
}

const fn choice(
    section: &'static str,
    key: &'static str,
    label: &'static str,
    hint: &'static str,
    choices: &'static [&'static str],
) -> Option_ {
    Option_ { key, section, label, hint, kind: "choice", choices, min: 0, max: 0 }
}

const fn number(
    section: &'static str,
    key: &'static str,
    label: &'static str,
    hint: &'static str,
    min: i64,
    max: i64,
) -> Option_ {
    Option_ { key, section, label, hint, kind: "number", choices: &[], min, max }
}

const fn switch(
    section: &'static str,
    key: &'static str,
    label: &'static str,
    hint: &'static str,
) -> Option_ {
    Option_ { key, section, label, hint, kind: "switch", choices: &[], min: 0, max: 0 }
}

/// The settings worth putting in front of someone trying to make a game behave.
/// Not all three hundred RPCS3 has: the rest are debugging switches that do
/// nothing good in ordinary hands.
pub const OPTIONS: &[Option_] = &[
    // --- Video ---
    choice("Video", "Renderer", "Renderer", "Vulkan suits nearly everything. OpenGL is worth trying when a game will not draw.", &["Vulkan", "OpenGL", "Null"]),
    number("Video", "Resolution Scale", "Resolution scale", "Percent of native. Above 100 sharpens the picture and costs GPU.", 50, 300),
    choice("Video", "Frame limit", "Frame limit", "Some games misbehave when they run faster than they expect.", &["Off", "30", "50", "60", "120", "Display", "Auto", "PS3 Native", "Infinite"]),
    choice("Video", "VSync Mode", "VSync", "Stops tearing at the cost of a little latency.", &["Disabled", "Adaptive", "Full"]),
    choice("Video", "Aspect ratio", "Aspect ratio", "What the game is told the screen shape is.", &["16:9", "4:3"]),
    choice("Video", "Shader Mode", "Shader mode", "The interpreter variants stutter less on first run and cost speed.", &["Async Recompiler with Shader Interpreter", "Async Recompiler (multi-threaded)", "Legacy Recompiler (single-threaded)", "Shader Interpreter only"]),
    choice("Video", "Output Scaling Mode", "Output scaling", "How the picture is stretched to your window.", &["FidelityFX Super Resolution", "Bilinear", "Nearest"]),
    choice("Video", "MSAA", "Anti-aliasing", "Smooths edges. Turn off if a game shows artefacts.", &["Auto", "Disabled"]),
    number("Video", "Anisotropic Filter Override", "Anisotropic filtering", "Sharpens textures seen at an angle. 0 leaves it to the game.", 0, 16),
    switch("Video", "Write Color Buffers", "Write colour buffers", "Needed by some games for effects to appear. Costs speed."),
    switch("Video", "Strict Rendering Mode", "Strict rendering", "Slow, but fixes games that render wrongly otherwise."),
    switch("Video", "Multithreaded RSX", "Multithreaded RSX", "Can help on many-core machines and hurt on few."),
    // --- CPU ---
    choice("Core", "PPU Decoder", "PPU decoder", "The recompiler is far faster. Interpreters are for when a game will not run at all.", &["Recompiler (LLVM)", "Interpreter (static)"]),
    choice("Core", "SPU Decoder", "SPU decoder", "As above, for the PS3's other processors.", &["Recompiler (LLVM)", "Recompiler (ASMJIT)", "Interpreter (dynamic)", "Interpreter (static)"]),
    choice("Core", "SPU Block Size", "SPU block size", "Larger blocks can be faster and can break games.", &["Safe", "Mega", "Giga"]),
    choice("Core", "SPU XFloat Accuracy", "SPU float accuracy", "Lower accuracy is faster and can corrupt graphics or physics.", &["Accurate", "Approximate", "Relaxed", "Inaccurate"]),
    number("Core", "Preferred SPU Threads", "Preferred SPU threads", "0 lets RPCS3 decide, which is usually right.", 0, 6),
    switch("Core", "SPU loop detection", "SPU loop detection", "Occasionally helps games that stutter."),
    // --- Audio ---
    choice("Audio", "Renderer", "Audio backend", "Try another if sound crackles or is silent.", &["Cubeb", "XAudio2", "FAudio", "Null"]),
    number("Audio", "Master Volume", "Volume", "Percent.", 0, 200),
    switch("Audio", "Enable Buffering", "Audio buffering", "Smoother sound, slightly more delay."),
    switch("Audio", "Enable Time Stretching", "Time stretching", "Keeps audio pitch steady when the game runs slow."),
];

fn config_path(app: &AppHandle, title_id: &str) -> Result<PathBuf, String> {
    Ok(super::install_dir(app)?
        .join("config")
        .join("custom_configs")
        .join(format!("config_{title_id}.yml")))
}

/// What the user has chosen for this game, as section -> key -> value. Anything
/// absent is RPCS3's own default and is deliberately not represented here.
pub type Chosen = BTreeMap<String, BTreeMap<String, String>>;

pub fn read(app: &AppHandle, title_id: &str) -> Chosen {
    let Ok(text) = std::fs::read_to_string(config_path(app, title_id).unwrap_or_default()) else {
        return Chosen::new();
    };
    parse(&text)
}

/// The file is ours: two levels, no lists, no comments. Anything with a deeper
/// shape was not written by Omoio and is left out rather than half-understood.
fn parse(text: &str) -> Chosen {
    let mut chosen = Chosen::new();
    let mut section = String::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if !line.starts_with(' ') {
            section = line.trim_end().trim_end_matches(':').to_string();
            continue;
        }
        if section.is_empty() {
            continue;
        }
        let Some((key, value)) = line.trim().split_once(": ") else {
            continue;
        };
        chosen
            .entry(section.clone())
            .or_default()
            .insert(key.trim().to_string(), unquote(value.trim()));
    }
    chosen
}

fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(value)
        .to_string()
}

/// Writes only what was chosen. An empty set removes the file entirely, so the
/// game goes back to being run exactly as RPCS3 would run it.
pub fn write(app: &AppHandle, title_id: &str, chosen: &Chosen) -> Result<(), String> {
    let path = config_path(app, title_id)?;
    let meaningful: Chosen = chosen
        .iter()
        .filter(|(_, keys)| !keys.is_empty())
        .map(|(s, k)| (s.clone(), k.clone()))
        .collect();

    if meaningful.is_empty() {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut out = String::new();
    for (section, keys) in &meaningful {
        out.push_str(section);
        out.push_str(":\n");
        for (key, value) in keys {
            out.push_str(&format!("  {key}: {}\n", quote_if_needed(value)));
        }
    }
    std::fs::write(&path, out).map_err(|e| e.to_string())
}

/// Numbers and booleans go bare; anything else is quoted so a value like "Null"
/// stays a string rather than becoming YAML's null.
fn quote_if_needed(value: &str) -> String {
    let bare = value == "true"
        || value == "false"
        || (!value.is_empty() && value.chars().all(|c| c.is_ascii_digit()));
    if bare {
        value.to_string()
    } else {
        format!("\"{}\"", value.replace('"', "\\\""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_back_what_it_writes() {
        let text = "Video:\n  Renderer: \"OpenGL\"\n  Resolution Scale: 150\nCore:\n  SPU Block Size: \"Mega\"\n";
        let chosen = parse(text);
        assert_eq!(chosen["Video"]["Renderer"], "OpenGL");
        assert_eq!(chosen["Video"]["Resolution Scale"], "150");
        assert_eq!(chosen["Core"]["SPU Block Size"], "Mega");
    }

    #[test]
    fn leaves_numbers_and_booleans_bare_and_quotes_words() {
        assert_eq!(quote_if_needed("150"), "150");
        assert_eq!(quote_if_needed("true"), "true");
        assert_eq!(quote_if_needed("false"), "false");
        // Unquoted, YAML would read this as nothing at all.
        assert_eq!(quote_if_needed("Null"), "\"Null\"");
        assert_eq!(quote_if_needed("Recompiler (LLVM)"), "\"Recompiler (LLVM)\"");
    }

    #[test]
    fn ignores_shapes_it_did_not_write() {
        // RPCS3's own dump nests and lists; none of that is ours to interpret.
        let chosen = parse("Video:\n  Vulkan:\n    Adapter: NVIDIA\n  Renderer: \"Vulkan\"\n");
        assert_eq!(chosen["Video"]["Renderer"], "Vulkan");
        assert!(!chosen["Video"].contains_key("Vulkan"));
    }

    #[test]
    fn every_option_offered_belongs_to_a_section_rpcs3_has() {
        for option in OPTIONS {
            assert!(
                matches!(option.section, "Video" | "Core" | "Audio"),
                "{} is in an unknown section",
                option.key
            );
            assert!(!option.label.is_empty());
            assert!(!option.hint.is_empty(), "{} needs to say what it does", option.key);
        }
    }

    #[test]
    fn choice_options_offer_choices_and_numbers_offer_a_range() {
        for option in OPTIONS {
            match option.kind {
                "choice" => assert!(!option.choices.is_empty(), "{} has no values", option.key),
                "number" => assert!(option.max > option.min, "{} has no range", option.key),
                "switch" => {}
                other => panic!("{} has an unknown kind {other}", option.key),
            }
        }
    }

    #[test]
    fn round_trips_through_the_file_format() {
        let mut chosen = Chosen::new();
        chosen.entry("Video".into()).or_default().insert("Renderer".into(), "OpenGL".into());
        chosen.entry("Video".into()).or_default().insert("Resolution Scale".into(), "150".into());

        let mut out = String::new();
        for (section, keys) in &chosen {
            out.push_str(section);
            out.push_str(":\n");
            for (key, value) in keys {
                out.push_str(&format!("  {key}: {}\n", quote_if_needed(value)));
            }
        }
        assert_eq!(parse(&out), chosen);
    }
}

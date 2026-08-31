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
//!
//! The list of settings is not ours either. It is read out of RPCS3's own
//! `config.yml`, so every setting the installed build has is one Omoio can set,
//! and a build that gains a setting next week gains it here too. A handful get
//! a friendlier name and an explanation on top of that; the rest are shown
//! under the names RPCS3 uses, which are the names anyone advising you will
//! use as well.

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tauri::AppHandle;

/// Settings nest, and one of RPCS3's own section names contains a slash
/// ("Input/Output"), so paths are joined on a character that cannot appear in
/// one rather than on any kind of punctuation.
const SEP: &str = "\n";

/// A setting as the interface should present it: RPCS3's own key and default,
/// plus our own words where we have them.
#[derive(Serialize)]
pub struct Setting {
    pub key: String,
    pub group: String,
    pub name: String,
    pub label: String,
    pub hint: String,
    pub kind: String,
    pub choices: &'static [&'static str],
    pub default: String,
    pub min: i64,
    pub max: i64,
    pub common: bool,
}

struct Curated {
    section: &'static str,
    key: &'static str,
    label: &'static str,
    hint: &'static str,
    choices: &'static [&'static str],
    min: i64,
    max: i64,
}

const fn choice(
    section: &'static str,
    key: &'static str,
    label: &'static str,
    hint: &'static str,
    choices: &'static [&'static str],
) -> Curated {
    Curated { section, key, label, hint, choices, min: 0, max: 0 }
}

const fn number(
    section: &'static str,
    key: &'static str,
    label: &'static str,
    hint: &'static str,
    min: i64,
    max: i64,
) -> Curated {
    Curated { section, key, label, hint, choices: &[], min, max }
}

const fn switch(
    section: &'static str,
    key: &'static str,
    label: &'static str,
    hint: &'static str,
) -> Curated {
    Curated { section, key, label, hint, choices: &[], min: 0, max: 0 }
}

/// The settings worth putting in front of someone who just wants a game to
/// behave. Everything else is still reachable, one tab over. Every option list
/// here comes from RPCS3's own system_config_types.cpp, and a test checks that
/// each of these keys exists in the installed build.
const CURATED: &[Curated] = &[
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
    choice("Core", "PPU Decoder", "PPU decoder", "The recompiler is far faster. Interpreters are for when a game will not run at all.", &["Recompiler (LLVM)", "Interpreter (static)"]),
    choice("Core", "SPU Decoder", "SPU decoder", "As above, for the PS3's other processors.", &["Recompiler (LLVM)", "Recompiler (ASMJIT)", "Interpreter (dynamic)", "Interpreter (static)"]),
    choice("Core", "SPU Block Size", "SPU block size", "Larger blocks can be faster and can break games.", &["Safe", "Mega", "Giga"]),
    choice("Core", "SPU XFloat Accuracy", "SPU float accuracy", "Lower accuracy is faster and can corrupt graphics or physics.", &["Accurate", "Approximate", "Relaxed", "Inaccurate"]),
    number("Core", "Preferred SPU Threads", "Preferred SPU threads", "0 lets RPCS3 decide, which is usually right.", 0, 6),
    switch("Core", "SPU loop detection", "SPU loop detection", "Occasionally helps games that stutter."),
    choice("Audio", "Renderer", "Audio backend", "Try another if sound crackles or is silent.", &["Cubeb", "XAudio2", "FAudio", "Null"]),
    number("Audio", "Master Volume", "Volume", "Percent.", 0, 200),
    switch("Audio", "Enable Buffering", "Audio buffering", "Smoother sound, slightly more delay."),
    switch("Audio", "Enable Time Stretching", "Time stretching", "Keeps audio pitch steady when the game runs slow."),
];

fn curated_for(key: &str) -> Option<&'static Curated> {
    CURATED.iter().find(|c| {
        key.strip_prefix(c.section)
            .and_then(|rest| rest.strip_prefix(SEP))
            .is_some_and(|leaf| leaf == c.key)
    })
}

fn config_path(app: &AppHandle, title_id: &str) -> Result<PathBuf, String> {
    Ok(super::install_dir(app)?
        .join("config")
        .join("custom_configs")
        .join(format!("config_{title_id}.yml")))
}

fn defaults_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(super::install_dir(app)?.join("config").join("config.yml"))
}

/// What the user has chosen for this game, keyed by the setting's full path.
/// Anything absent is RPCS3's own default and is deliberately not represented.
pub type Chosen = BTreeMap<String, String>;

/// Every setting the installed RPCS3 has, in the order it lists them.
///
/// Empty when RPCS3 has not written its config yet, which happens only before
/// it has ever run. The interface says so rather than showing a bare list.
pub fn catalogue(app: &AppHandle) -> Vec<Setting> {
    let Ok(text) = defaults_path(app).and_then(|p| std::fs::read_to_string(p).map_err(|e| e.to_string()))
    else {
        return Vec::new();
    };

    entries(&text)
        .into_iter()
        .map(|(key, default)| {
            let curated = curated_for(&key);
            let mut parts: Vec<&str> = key.split(SEP).collect();
            let name = parts.pop().unwrap_or_default().to_string();
            let group = parts.join(" / ");

            let kind = if curated.is_some_and(|c| !c.choices.is_empty()) {
                "choice"
            } else if default == "true" || default == "false" {
                "switch"
            } else if default.parse::<i64>().is_ok() {
                "number"
            } else {
                "text"
            };

            Setting {
                label: curated.map_or_else(|| name.clone(), |c| c.label.to_string()),
                hint: curated.map_or(String::new(), |c| c.hint.to_string()),
                choices: curated.map(|c| c.choices).unwrap_or(&[]),
                min: curated.map_or(0, |c| c.min),
                max: curated.map_or(0, |c| c.max),
                common: curated.is_some(),
                kind: kind.to_string(),
                key,
                group,
                name,
                default,
            }
        })
        .collect()
}

pub fn read(app: &AppHandle, title_id: &str) -> Chosen {
    let Ok(text) = std::fs::read_to_string(config_path(app, title_id).unwrap_or_default()) else {
        return Chosen::new();
    };
    entries(&text).into_iter().collect()
}

/// Reads the subset of YAML that RPCS3 writes: nested maps of scalars, two
/// spaces per level. Lists and matrices are skipped rather than half-understood,
/// so Omoio never rewrites something it cannot faithfully represent.
fn entries(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut stack: Vec<String> = Vec::new();

    for line in text.lines() {
        let body = line.trim();
        if body.is_empty() {
            continue;
        }
        let depth = (line.len() - line.trim_start().len()) / 2;

        // A bare [] or {} is the value of the key above it, which is a list.
        if body == "[]" || body == "{}" {
            stack.truncate(depth.saturating_sub(1));
            continue;
        }

        stack.truncate(depth);
        match body.split_once(": ") {
            Some((key, value)) => {
                stack.push(key.trim().to_string());
                out.push((stack.join(SEP), unquote(value.trim())));
                stack.pop();
            }
            None => stack.push(body.trim_end_matches(':').to_string()),
        }
    }
    out
}

fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .map(|v| v.replace("\\\"", "\""))
        .unwrap_or_else(|| value.to_string())
}

/// Writes only what was chosen. An empty set removes the file entirely, so the
/// game goes back to being run exactly as RPCS3 would run it.
pub fn write(app: &AppHandle, title_id: &str, chosen: &Chosen) -> Result<(), String> {
    let path = config_path(app, title_id)?;

    if chosen.is_empty() {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, render(chosen)).map_err(|e| e.to_string())
}

fn render(chosen: &Chosen) -> String {
    let paths: Vec<(Vec<&str>, &String)> = chosen
        .iter()
        .map(|(key, value)| (key.split(SEP).collect(), value))
        .collect();
    let mut out = String::new();
    emit(&mut out, &paths, 0);
    out
}

fn emit(out: &mut String, entries: &[(Vec<&str>, &String)], depth: usize) {
    let mut i = 0;
    while i < entries.len() {
        let head = entries[i].0[depth];
        let mut end = i;
        while end < entries.len() && entries[end].0[depth] == head {
            end += 1;
        }
        let pad = "  ".repeat(depth);
        if entries[i].0.len() == depth + 1 {
            out.push_str(&format!("{pad}{head}: {}\n", quote_if_needed(entries[i].1)));
        } else {
            out.push_str(&format!("{pad}{head}:\n"));
            emit(out, &entries[i..end], depth + 1);
        }
        i = end;
    }
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

    fn chosen(pairs: &[(&str, &str)]) -> Chosen {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn reads_back_what_it_writes() {
        let text = "Video:\n  Renderer: \"OpenGL\"\n  Resolution Scale: 150\nCore:\n  SPU Block Size: \"Mega\"\n";
        let map: Chosen = entries(text).into_iter().collect();
        assert_eq!(map["Video\nRenderer"], "OpenGL");
        assert_eq!(map["Video\nResolution Scale"], "150");
        assert_eq!(map["Core\nSPU Block Size"], "Mega");
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
    fn keeps_values_that_contain_a_colon() {
        // "Aspect ratio: 16:9" and "GDB Server: 127.0.0.1:2345" both appear in
        // RPCS3's own config.
        let map: Chosen = entries("Video:\n  Aspect ratio: 16:9\n").into_iter().collect();
        assert_eq!(map["Video\nAspect ratio"], "16:9");
    }

    #[test]
    fn reaches_settings_nested_below_a_section() {
        let map: Chosen =
            entries("Video:\n  Renderer: Vulkan\n  Vulkan:\n    Adapter: NVIDIA\n    Use Re-BAR for GPU uploads: true\n")
                .into_iter()
                .collect();
        assert_eq!(map["Video\nVulkan\nAdapter"], "NVIDIA");
        assert_eq!(map["Video\nVulkan\nUse Re-BAR for GPU uploads"], "true");
        assert_eq!(map["Video\nRenderer"], "Vulkan");
    }

    #[test]
    fn skips_lists_without_swallowing_what_follows() {
        let map: Chosen =
            entries("Core:\n  Libraries Control:\n    []\n  HLE lwmutex: false\n").into_iter().collect();
        assert!(!map.keys().any(|k| k.contains("Libraries Control")));
        assert_eq!(map["Core\nHLE lwmutex"], "false");
    }

    #[test]
    fn round_trips_through_the_file_format() {
        let start = chosen(&[
            ("Video\nRenderer", "OpenGL"),
            ("Video\nVulkan\nAdapter", "NVIDIA RTX A4500"),
            ("Core\nSPU Block Size", "Mega"),
        ]);
        let map: Chosen = entries(&render(&start)).into_iter().collect();
        assert_eq!(map, start);
    }

    #[test]
    fn nests_the_file_the_way_rpcs3_nests_its_own() {
        let out = render(&chosen(&[
            ("Video\nRenderer", "OpenGL"),
            ("Video\nVulkan\nAdapter", "NVIDIA"),
        ]));
        assert_eq!(out, "Video:\n  Renderer: \"OpenGL\"\n  Vulkan:\n    Adapter: \"NVIDIA\"\n");
    }

    #[test]
    fn curated_entries_are_matched_by_their_whole_path() {
        assert!(curated_for("Video\nRenderer").is_some());
        assert!(curated_for("Audio\nRenderer").is_some());
        // Same leaf name, different section: must not borrow Video's wording.
        assert_eq!(curated_for("Audio\nRenderer").unwrap().label, "Audio backend");
        assert!(curated_for("Video\nVulkan\nAdapter").is_none());
    }

    /// The whole point of reading RPCS3's own config is that we stop guessing.
    /// This checks the curated names against the installed build; set
    /// OMOIO_RPCS3_CONFIG to its config.yml to run it.
    #[test]
    #[ignore]
    fn every_curated_key_exists_in_the_installed_build() {
        let path = std::env::var("OMOIO_RPCS3_CONFIG").expect("set OMOIO_RPCS3_CONFIG");
        let text = std::fs::read_to_string(path).expect("read config.yml");
        let keys: Vec<String> = entries(&text).into_iter().map(|(k, _)| k).collect();
        for c in CURATED {
            let want = format!("{}{}{}", c.section, SEP, c.key);
            assert!(keys.contains(&want), "{} / {} is not a setting RPCS3 has", c.section, c.key);
        }
    }
}

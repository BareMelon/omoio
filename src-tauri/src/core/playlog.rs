//! Keeping what RPCS3 said about a game after it stops.
//!
//! RPCS3 overwrites its log on every launch, so a crash is erased the moment
//! anything is started again. Omoio copies each session out before that
//! happens, which is the whole point: you cannot look into a crash whose
//! evidence is gone.

use serde::{Deserialize, Serialize};

/// What RPCS3 writes in front of every line. Taken from its own logs.cpp
/// rather than guessed, because the whole reading of a session rests on it.
const FATAL: &str = "·F";
const ERROR: &str = "·E";
const WARNING: &str = "·W";

/// How a session ended. Only `Crashed` claims a crash, and only on evidence:
/// a fatal line or a bad exit. Closing a game is not a crash, and telling
/// someone their game crashed when they closed it is worse than saying nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ending {
    Stopped,
    Closed,
    Crashed,
}

impl Ending {
    pub fn describe(self) -> &'static str {
        match self {
            Self::Stopped => "Stopped from Omoio",
            Self::Closed => "Closed normally",
            Self::Crashed => "Ended unexpectedly",
        }
    }
}

/// The machine this ran on, lifted from the header RPCS3 writes. Exactly what
/// anyone diagnosing a problem asks for first.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Machine {
    pub rpcs3: Option<String>,
    pub cpu: Option<String>,
    pub os: Option<String>,
    pub gpu: Option<String>,
    pub renderer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub title_id: String,
    pub title: String,
    pub started: String,
    pub seconds: u64,
    pub ending: Ending,
    pub machine: Machine,
    /// The lines worth reading. A session runs to thousands of lines and a
    /// handful matter.
    pub problems: Vec<String>,
    pub log_file: String,
}

/// RPCS3's header is the first few lines, before any timestamped entry.
pub fn read_machine(log: &str) -> Machine {
    let mut machine = Machine::default();
    for line in log.lines().take(40) {
        let line = line.trim_start_matches('\u{feff}');
        if machine.rpcs3.is_none() && line.starts_with("RPCS3 v") {
            machine.rpcs3 = Some(line.trim().to_string());
        } else if line.starts_with("Operating system:") {
            machine.os = Some(after_colon(line));
        } else if line.contains(" Threads |") {
            // "AMD EPYC 7543P | 8 Threads | 27.98 GiB RAM | ... | AVX+ | FMA3"
            machine.cpu = Some(line.trim().to_string());
        } else if line.contains("Found Vulkan-compatible GPU:") {
            machine.gpu = line.split_once("GPU: ").map(|(_, g)| g.trim().to_string());
        } else if line.contains("Setting the default renderer to") {
            // "...renderer to Vulkan. Default GPU: '...'" - the sentence ends
            // at the stop, and the rest is about the card, not the renderer.
            machine.renderer = line
                .split_once("renderer to ")
                .map(|(_, rest)| rest.split('.').next().unwrap_or(rest).trim().to_string());
        }
    }
    machine
}

fn after_colon(line: &str) -> String {
    line.split_once(':')
        .map(|(_, rest)| rest.trim().to_string())
        .unwrap_or_else(|| line.trim().to_string())
}

/// Fatals and errors, in order, deduplicated. RPCS3 repeats the same failure
/// for every frame it happens on, and fifty copies of one line is not fifty
/// problems.
pub fn read_problems(log: &str, limit: usize) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut problems = Vec::new();
    for line in log.lines() {
        if !line.starts_with(FATAL) && !line.starts_with(ERROR) {
            continue;
        }
        // The timestamp differs every time; compare on what was said.
        let body = line.splitn(3, ' ').nth(2).unwrap_or(line).to_string();
        if seen.insert(body.clone()) {
            problems.push(line.trim().to_string());
            if problems.len() >= limit {
                break;
            }
        }
    }
    problems
}

pub fn has_fatal(log: &str) -> bool {
    log.lines().any(|line| line.starts_with(FATAL))
}

pub fn count_warnings(log: &str) -> usize {
    log.lines().filter(|line| line.starts_with(WARNING)).count()
}

/// A question someone can paste somewhere and get a useful answer to, built
/// only from what the log and library actually say.
pub fn troubleshooting_prompt(session: &Session) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "I'm running the PS3 game {} ({}) in RPCS3 and need help working out what went wrong.\n\n",
        session.title, session.title_id
    ));
    out.push_str(&format!(
        "The session {} after {}.\n\n",
        match session.ending {
            Ending::Crashed => "ended unexpectedly",
            Ending::Closed => "closed normally",
            Ending::Stopped => "was stopped by me",
        },
        format_duration(session.seconds)
    ));

    out.push_str("Setup:\n");
    for (label, value) in [
        ("RPCS3", &session.machine.rpcs3),
        ("CPU", &session.machine.cpu),
        ("GPU", &session.machine.gpu),
        ("Renderer", &session.machine.renderer),
        ("OS", &session.machine.os),
    ] {
        if let Some(value) = value {
            out.push_str(&format!("- {label}: {value}\n"));
        }
    }

    if session.problems.is_empty() {
        out.push_str("\nRPCS3 logged no errors for this session.\n");
    } else {
        out.push_str(&format!("\nWhat RPCS3 logged ({}):\n", session.problems.len()));
        for problem in &session.problems {
            out.push_str(&format!("{problem}\n"));
        }
    }
    out.push_str("\nWhat is likely causing this, and what should I change?\n");
    out
}

pub fn format_duration(seconds: u64) -> String {
    match seconds {
        0..=59 => format!("{seconds}s"),
        60..=3599 => format!("{}m {}s", seconds / 60, seconds % 60),
        _ => format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shaped like the real header, taken from an actual session on this machine.
    const HEADER: &str = "\u{feff}RPCS3 v0.0.42-19884-3ef20ebb Alpha | master\n\
Architecture: x64\n\
AMD EPYC 7543P 32-Core Processor | 8 Threads | 27.98 GiB RAM | TSC: 2.800GHz | AVX+ | FMA3\n\
Operating system: Windows, Major: 10, Minor: 0, Build: 22621, Service Pack: none\n\
Qt version: Compiled against Qt 6.11.2\n\
·A 0:00:00.155719 {Vulkan Device Enumeration Thread} RSX: Found Vulkan-compatible GPU: 'NVIDIA RTX A4500' running on driver 597.6.0.0\n\
·! 0:00:00.156589 CFG: Setting the default renderer to Vulkan. Default GPU: 'NVIDIA RTX A4500'\n";

    #[test]
    fn lifts_the_machine_out_of_the_header() {
        let machine = read_machine(HEADER);
        assert_eq!(machine.rpcs3.as_deref(), Some("RPCS3 v0.0.42-19884-3ef20ebb Alpha | master"));
        assert!(machine.cpu.as_deref().unwrap().contains("AVX+"));
        assert!(machine.os.as_deref().unwrap().starts_with("Windows"));
        assert!(machine.gpu.as_deref().unwrap().contains("RTX A4500"));
        assert!(machine.gpu.as_deref().unwrap().contains("597.6.0.0"));
        assert_eq!(machine.renderer.as_deref(), Some("Vulkan"));
    }

    #[test]
    fn a_header_it_cannot_read_leaves_the_fields_empty() {
        let machine = read_machine("nothing useful here\nnor here\n");
        assert!(machine.rpcs3.is_none());
        assert!(machine.gpu.is_none());
    }

    #[test]
    fn keeps_errors_and_fatals_and_leaves_the_noise() {
        let log = "·! 0:00:01.0 SYS: starting\n\
·W 0:00:02.0 sys_fs: a warning\n\
·E 0:00:03.0 sys_fs: could not open thing\n\
·T 0:00:04.0 trace noise\n\
·F 0:00:05.0 RSX: it fell over\n";
        let problems = read_problems(log, 20);
        assert_eq!(problems.len(), 2);
        assert!(problems[0].contains("could not open thing"));
        assert!(problems[1].contains("it fell over"));
    }

    #[test]
    fn one_failure_repeated_every_frame_is_still_one_problem() {
        let log = "·E 0:00:01.0 RSX: same thing\n\
·E 0:00:02.5 RSX: same thing\n\
·E 0:00:09.9 RSX: same thing\n\
·E 0:00:11.0 RSX: a different thing\n";
        assert_eq!(read_problems(log, 20).len(), 2);
    }

    #[test]
    fn stops_collecting_at_the_limit() {
        let log: String = (0..50).map(|i| format!("·E 0:00:0{i}.0 problem {i}\n")).collect();
        assert_eq!(read_problems(&log, 10).len(), 10);
    }

    #[test]
    fn a_crash_is_only_claimed_when_something_fatal_was_logged() {
        assert!(has_fatal("·! fine\n·F 0:00:01.0 RSX: it fell over\n"));
        // A session full of errors that still ended cleanly is not a crash.
        assert!(!has_fatal("·E 0:00:01.0 sys_fs: missing optional file\n·W 0:00:02.0 warning\n"));
        assert!(!has_fatal(""));
    }

    #[test]
    fn counts_warnings_without_counting_anything_else() {
        assert_eq!(count_warnings("·W a\n·E b\n·W c\n·! d\n"), 2);
    }

    #[test]
    fn writes_a_prompt_from_what_was_actually_recorded() {
        let session = Session {
            title_id: "BCES00850".into(),
            title: "LittleBigPlanet 2".into(),
            started: "2026-08-31T09:20:01".into(),
            seconds: 754,
            ending: Ending::Crashed,
            machine: read_machine(HEADER),
            problems: vec!["·F 0:00:05.0 RSX: it fell over".into()],
            log_file: "logs/BCES00850-1.log".into(),
        };
        let prompt = troubleshooting_prompt(&session);

        assert!(prompt.contains("LittleBigPlanet 2"));
        assert!(prompt.contains("BCES00850"));
        assert!(prompt.contains("ended unexpectedly"));
        assert!(prompt.contains("12m 34s"));
        assert!(prompt.contains("RTX A4500"));
        assert!(prompt.contains("AVX+"));
        assert!(prompt.contains("it fell over"));
    }

    #[test]
    fn says_so_plainly_when_there_was_nothing_to_report() {
        let session = Session {
            title_id: "BCES00850".into(),
            title: "LittleBigPlanet 2".into(),
            started: "2026-08-31T09:20:01".into(),
            seconds: 30,
            ending: Ending::Closed,
            machine: Machine::default(),
            problems: vec![],
            log_file: "logs/x.log".into(),
        };
        assert!(troubleshooting_prompt(&session).contains("logged no errors"));
    }

    #[test]
    fn reads_durations_the_way_a_person_would_say_them() {
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(754), "12m 34s");
        assert_eq!(format_duration(3661), "1h 1m");
    }
}

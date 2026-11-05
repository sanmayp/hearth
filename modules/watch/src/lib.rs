//! Watch module. Board for the Muse build sessions.
//! Endpoints parse the watcher log. The only write is the ask-me
//! queue: owner answers wait there, nothing sends to any thread.

use async_trait::async_trait;
use axum::{
    extract::Path as AxumPath,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use hearth_core::{AppState, Health, JobDef, Migration, Module, ToolDef, WidgetDef};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};

/// The watch module.
pub struct Watch;

/// States the watcher writes. Anything else in the name slot is a name.
fn is_known_state(s: &str) -> bool {
    matches!(s, "working" | "idle" | "blocked" | "done" | "gone")
}

/// One session row on the board.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionStatus {
    /// Session name, e.g. "lifeos".
    pub name: String,
    /// Herdr pane id, e.g. "w1:p19".
    pub pane: String,
    /// Current state: working, idle, blocked, done, gone.
    pub state: String,
    /// Last log line time, if the pane ever appeared.
    pub at: Option<String>,
    /// Time the current state began, if known.
    pub changed_at: Option<String>,
    /// Alerts in plain words.
    pub alerts: Vec<String>,
    /// Latest stage report, when a file carries one.
    pub stage: Option<String>,
    /// Latest open question, when the session asked one.
    pub question: Option<String>,
    /// Latest recommendation, when the session gave one.
    pub recommendation: Option<String>,
}

/// Whole board: five sessions plus the newest log time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Board {
    /// Sessions in board order.
    pub sessions: Vec<SessionStatus>,
    /// Newest log line time, for stale-since labels.
    pub log_at: Option<String>,
}

#[async_trait]
impl Module for Watch {
    fn id(&self) -> &'static str {
        "watch"
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![]
    }

    fn routes(&self, _state: &AppState) -> Router<()> {
        Router::new()
            .route("/v1/watch/muse", get(get_muse))
            .route("/v1/watch/threads", get(get_threads))
            .route("/v1/watch/needs-you", get(get_needs_you))
            .route("/v1/watch/features", get(get_features))
            .route("/v1/watch/features/:id/confirm", post(post_feature_confirm))
            .route("/v1/watch/features/:id/step", post(post_feature_step))
            .route("/v1/watch/screens", get(get_screens))
            .route("/v1/watch/screens/open", post(post_screen_open))
            .route("/v1/watch/screens/confirm", post(post_screen_confirm))
            .route("/v1/watch/dev/feed", get(get_dev_feed))
            .route("/v1/features/:id/mark", post(post_feature_mark))
            .route("/v1/watch/ask-queue", get(get_ask).post(post_ask))
    }

    fn jobs(&self) -> Vec<JobDef> {
        vec![]
    }

    fn tools(&self) -> Vec<ToolDef> {
        vec![]
    }

    fn widgets(&self) -> Vec<WidgetDef> {
        vec![
            WidgetDef {
                id: "watch.card",
                title: "Muse sessions",
                module: "watch",
            },
            WidgetDef {
                id: "watch.threads",
                title: "Thread board",
                module: "watch",
            },
        ]
    }

    async fn health(&self, _state: &AppState) -> Health {
        match std::fs::metadata(log_path()) {
            Ok(_) => Health::ok("watch log reads"),
            Err(e) => Health::down(format!("watch log down: {e}")),
        }
    }
}

/// Serve the board. Reads the log file on each call.
async fn get_muse() -> impl IntoResponse {
    let text = match std::fs::read_to_string(log_path()) {
        Ok(t) => t,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"message": "Watch log is missing. Retry soon."})),
            )
                .into_response();
        }
    };
    Json(json!(parse_log(&text))).into_response()
}

/// Log file path. Env wins, else the watcher default.
fn log_path() -> PathBuf {
    if let Ok(p) = std::env::var("MUSE_WATCH_LOG") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("hearth/MUSE-WATCH-LOG.md")
}

/// One parsed pane entry: time, state, raw alert tokens.
type Entry = (String, String, Vec<String>);

/// Split one cell into pane, state, alerts, plus an optional name.
/// Old cells read `pane state`; new cells read `pane name state`.
/// A name that equals a state still parses: two states in a row
/// mean the first one is the name.
fn split_cell(words: &[&str]) -> (String, String, Vec<String>, Option<String>) {
    let pane = words[0].to_string();
    if words.len() < 3 {
        return (pane, words[1].to_string(), Vec::new(), None);
    }
    if is_known_state(words[1]) && !is_known_state(words[2]) {
        let raw = words[2..].iter().map(|w| w.to_string()).collect();
        (pane, words[1].to_string(), raw, None)
    } else {
        let raw = words[3..].iter().map(|w| w.to_string()).collect();
        (pane, words[2].to_string(), raw, Some(words[1].to_string()))
    }
}

/// All pane entries plus latest names and newest stamp. Pure.
#[allow(clippy::type_complexity)]
fn collect_entries(
    text: &str,
) -> (
    Option<String>,
    std::collections::HashMap<String, Vec<Entry>>,
    std::collections::HashMap<String, String>,
) {
    let mut log_at: Option<String> = None;
    let mut by_pane: std::collections::HashMap<String, Vec<Entry>> =
        std::collections::HashMap::new();
    let mut names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for line in text.lines() {
        let Some((ts, rest)) = split_line(line) else {
            continue;
        };
        log_at = Some(ts.clone());
        for cell in rest.split('|') {
            let words: Vec<&str> = cell.split_whitespace().collect();
            if words.len() < 2 {
                continue;
            }
            let (pane, state, raw, name) = split_cell(&words);
            by_pane
                .entry(pane.clone())
                .or_default()
                .push((ts.clone(), state, raw));
            if let Some(n) = name {
                names.insert(pane, n);
            }
        }
    }
    (log_at, by_pane, names)
}

/// Days since civil epoch for a date. Howard Hinnant's algorithm.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Minutes for a log stamp. None when unreadable.
fn stamp_min(ts: &str) -> Option<i64> {
    let b = ts.as_bytes();
    if b.len() < 16 || (b[10] != b' ' && b[10] != b'T') {
        return None;
    }
    let num = |i: usize, n: usize| ts.get(i..i + n)?.parse::<i64>().ok();
    let (y, mo, d, h, mi) = (num(0, 4)?, num(5, 2)?, num(8, 2)?, num(11, 2)?, num(14, 2)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 {
        return None;
    }
    Some(days_from_civil(y, mo, d) * 1440 + h * 60 + mi)
}

/// Time the current state run began, ignoring alert changes.
/// Walks back while the state matches.
fn state_since(entries: &[Entry], state: &str) -> String {
    let mut first = entries.last().map(|e| e.0.clone()).unwrap_or_default();
    for e in entries.iter().rev() {
        if e.1 == state {
            first = e.0.clone();
        } else {
            break;
        }
    }
    first
}

/// Parse the whole log into a board. Pure, so tests cover it.
/// Panes come from the lines themselves, newest names win, order
/// stays stable by pane id. Gone runs over an hour drop out, and
/// so do panes unseen for over an hour.
fn parse_log(text: &str) -> Board {
    let (log_at, by_pane, names) = collect_entries(text);
    let now = log_at.as_deref().and_then(stamp_min);
    let mut panes: Vec<String> = by_pane.keys().cloned().collect();
    panes.sort();
    let sessions: Vec<SessionStatus> = panes
        .into_iter()
        .filter_map(|pane| {
            let entries = by_pane.get(&pane)?;
            let (ts, st, raw) = entries.last()?;
            if let (Some(now), Some(seen)) = (now, stamp_min(ts)) {
                if now - seen > 65 {
                    return None;
                }
            }
            if st == "gone" {
                let since = state_since(entries, "gone");
                if let (Some(now), Some(from)) = (now, stamp_min(&since)) {
                    if now - from > 60 {
                        return None;
                    }
                }
            }
            let alerts = raw.iter().map(|a| plain_alert(a)).collect();
            let changed = changed_since(entries, st, raw);
            let question = approval_question(st, raw);
            Some(SessionStatus {
                name: names.get(&pane).cloned().unwrap_or_else(|| pane.clone()),
                pane,
                state: st.clone(),
                at: Some(ts.clone()),
                changed_at: Some(changed),
                alerts,
                stage: None,
                question,
                recommendation: None,
            })
        })
        .map(scrub)
        .collect();
    Board { sessions, log_at }
}

/// Redact token-looking strings in free text fields. Runs when
/// question sources land; wired now so nothing slips past later.
fn scrub(mut s: SessionStatus) -> SessionStatus {
    s.stage = s.stage.map(|t| redact(&t));
    s.question = s.question.map(|t| redact(&t));
    s.recommendation = s.recommendation.map(|t| redact(&t));
    s
}

/// True for chars that appear inside tokens and keys.
fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~' | '+' | '=')
}

/// Replace token-looking runs (20 or more token chars) with a marker.
/// Plain prose, paths, and URLs pass through: spaces, slashes, and
/// colons break runs before they reach the threshold.
fn redact(text: &str) -> String {
    fn flush(run: &mut String, out: &mut String) {
        if run.len() >= 20 {
            out.push_str("[redacted]");
        } else {
            out.push_str(run);
        }
        run.clear();
    }
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    for c in text.chars() {
        if is_token_char(c) {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// Split a log line into time plus pane cells. None for junk lines.
fn split_line(line: &str) -> Option<(String, String)> {
    let line = line.strip_prefix("- ")?;
    let (ts, rest) = line.split_once(" | ")?;
    if ts.len() < 16 {
        return None;
    }
    Some((ts.trim().to_string(), rest.to_string()))
}

/// Time the current run began. Walks back while entries match.
fn changed_since(entries: &[Entry], state: &str, raw: &[String]) -> String {
    let mut first = entries.last().map(|e| e.0.clone()).unwrap_or_default();
    for e in entries.iter().rev() {
        if e.1 == state && e.2 == raw {
            first = e.0.clone();
        } else {
            break;
        }
    }
    first
}

/// Question text when the latest entry shows an approval prompt.
/// None for every other state. The log carries no recommendations.
fn approval_question(state: &str, raw: &[String]) -> Option<String> {
    if state == "blocked" && raw.iter().any(|t| t.starts_with("BLOCKED")) {
        Some("Shows an approval prompt. Needs the owner answer to continue.".to_string())
    } else {
        None
    }
}

/// One thread on the flow board.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThreadFlow {
    /// Session name, e.g. "lifeos".
    pub name: String,
    /// Herdr pane id, e.g. "w1:p19".
    pub pane: String,
    /// Current state: working, idle, blocked, done, gone.
    pub state: String,
    /// Flow lane: need, building, paused, review.
    pub flow: String,
    /// Last log line time, if the pane ever appeared.
    pub at: Option<String>,
    /// Time the current state began, if known.
    pub changed_at: Option<String>,
    /// One plain-English line on what the thread does.
    pub line: String,
    /// Latest stage report, when a file carries one.
    pub stage: Option<String>,
    /// Latest open question, when the session asked one.
    pub question: Option<String>,
    /// Latest recommendation, when the session gave one.
    pub recommendation: Option<String>,
}

/// One card in the needs-answer queue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NeedAnswer {
    /// Thread that waits.
    pub thread: String,
    /// The question it shows.
    pub question: String,
    /// Its recommendation, when it gave one.
    pub recommendation: Option<String>,
}

/// Whole thread board: flows, needs-answer queue, newest log time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThreadBoard {
    /// Threads in board order.
    pub threads: Vec<ThreadFlow>,
    /// Threads that wait on the owner, in board order.
    pub needs_answer: Vec<NeedAnswer>,
    /// Newest log line time, for stale-since labels.
    pub log_at: Option<String>,
}

/// Flow lane plus plain-English line for one session.
/// An open question wins: blocked with approval shows yellow even
/// though blocked alone is red. Idle is calm: finished sessions
/// wait for the next task, they are not broken.
fn flow_of(s: &SessionStatus) -> (String, String) {
    if s.question.is_some() {
        return ("need".to_string(), "Waits on the owner answer.".to_string());
    }
    match s.state.as_str() {
        "working" => (
            "building".to_string(),
            "Builds in the background.".to_string(),
        ),
        "done" => ("review".to_string(), "Ready for review.".to_string()),
        "gone" => ("paused".to_string(), "Session gone.".to_string()),
        "blocked" => ("paused".to_string(), "Pauses. No prompt shows.".to_string()),
        _ => (
            "waiting".to_string(),
            "Waits for the next task.".to_string(),
        ),
    }
}

/// Board plus needs-answer queue from parsed sessions. Pure, so
/// tests cover it.
fn thread_board(board: &Board) -> ThreadBoard {
    let mut needs_answer = Vec::new();
    let threads = board
        .sessions
        .iter()
        .map(|s| {
            let (flow, line) = flow_of(s);
            if flow == "need" {
                if let Some(q) = &s.question {
                    needs_answer.push(NeedAnswer {
                        thread: s.name.clone(),
                        question: q.clone(),
                        recommendation: s.recommendation.clone(),
                    });
                }
            }
            ThreadFlow {
                name: s.name.clone(),
                pane: s.pane.clone(),
                state: s.state.clone(),
                flow,
                at: s.at.clone(),
                changed_at: s.changed_at.clone(),
                line,
                stage: s.stage.clone(),
                question: s.question.clone(),
                recommendation: s.recommendation.clone(),
            }
        })
        .collect();
    ThreadBoard {
        threads,
        needs_answer,
        log_at: board.log_at.clone(),
    }
}

/// Serve the thread board. Reads the log file on each call.
async fn get_threads() -> impl IntoResponse {
    let text = match std::fs::read_to_string(log_path()) {
        Ok(t) => t,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"message": "Watch log is missing. Retry soon."})),
            )
                .into_response();
        }
    };
    Json(json!(thread_board(&parse_log(&text)))).into_response()
}

/// One change event for the Needs You log strip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NeedEvent {
    /// Log line time, e.g. "2026-11-01 01:42".
    pub at: String,
    /// Short tag: NEED, WORK, DONE, STALL, PAUSE, GONE, IDLE.
    pub tag: String,
    /// One plain line on what changed.
    pub text: String,
}

/// One shipped feature with no recorded use. Hand-marked.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnusedFeature {
    /// Stable id, e.g. "u1".
    pub id: String,
    /// Feature name as the owner knows it.
    pub name: String,
    /// App route that opens it, e.g. "/builds".
    pub route: String,
    /// Ship date, YYYY-MM-DD.
    pub shipped_at: String,
    /// Why it sits here. "Unverified" when no use data exists.
    pub note: String,
}

/// Whole Needs You feed: threads, unused builds, events, log time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NeedsFeed {
    /// Sessions in board order.
    pub threads: Vec<SessionStatus>,
    /// Hand-marked unused builds.
    pub unused: Vec<UnusedFeature>,
    /// Newest change events first, capped.
    pub events: Vec<NeedEvent>,
    /// Newest log line time, for stale-since labels.
    pub log_at: Option<String>,
}

/// Newest events the feed keeps.
const MAX_EVENTS: usize = 8;

/// Tag for one entry. Mirrors the baked snapshot script order.
fn event_tag(state: &str, raw: &[String]) -> &'static str {
    if raw.iter().any(|t| t.starts_with("BLOCKED")) {
        return "NEED";
    }
    if state == "working" {
        return "WORK";
    }
    if state == "done" {
        return "DONE";
    }
    if raw.iter().any(|t| t == "IDLE>30min") {
        return "STALL";
    }
    if state == "blocked" {
        return "PAUSE";
    }
    if state == "gone" {
        return "GONE";
    }
    "IDLE"
}

/// One plain line per change. Real states only, no invented stages.
fn event_text(name: &str, state: &str, raw: &[String]) -> String {
    if raw.iter().any(|t| t.starts_with("BLOCKED")) {
        return format!("{name} waits on approval.");
    }
    if state == "working" && raw.iter().any(|t| t == "SAME_SCREEN_WHILE_WORKING") {
        return format!("{name} works. The screen stays still.");
    }
    match state {
        "working" => format!("{name} starts building."),
        "done" => format!("{name} finishes. Ready for review."),
        "blocked" => format!("{name} pauses. No prompt shows."),
        "gone" => format!("{name} leaves. Session gone."),
        _ if raw.iter().any(|t| t == "IDLE>30min") => {
            format!("{name} idles over 30 min.")
        }
        _ => format!("{name} idles."),
    }
}

/// Change events across panes, newest first. Pure, so tests cover it.
fn build_events(text: &str) -> Vec<NeedEvent> {
    let (_, by_pane, names) = collect_entries(text);
    let mut panes: Vec<String> = by_pane.keys().cloned().collect();
    panes.sort();
    let mut events = Vec::new();
    for pane in panes {
        let Some(entries) = by_pane.get(&pane) else {
            continue;
        };
        let name = names.get(&pane).cloned().unwrap_or_else(|| pane.clone());
        for i in 1..entries.len() {
            let p = &entries[i - 1];
            let e = &entries[i];
            if e.1 == p.1 && e.2 == p.2 {
                continue;
            }
            events.push(NeedEvent {
                at: e.0.clone(),
                tag: event_tag(&e.1, &e.2).to_string(),
                text: event_text(&name, &e.1, &e.2),
            });
        }
    }
    events.sort_by(|a, b| b.at.cmp(&a.at));
    events.truncate(MAX_EVENTS);
    events
}

/// Hand-marked unused list. Env wins, else the repo default.
fn unused_path() -> PathBuf {
    if let Ok(p) = std::env::var("BUILT_UNUSED_FILE") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("hearth/web/data/built-unused.json")
}

/// Read unused rows. A missing or broken file means none yet,
/// not an error. Broken rows drop out; the rest keep serving.
fn read_unused(path: &Path) -> Vec<UnusedFeature> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    let Some(rows) = value.get("features").and_then(|f| f.as_array()) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|r| serde_json::from_value::<UnusedFeature>(r.clone()).ok())
        .map(|mut u| {
            u.name = redact(&u.name);
            u.note = redact(&u.note);
            u
        })
        .collect()
}

/// Needs You feed from one log read. Pure shape, so tests cover it.
fn needs_feed(text: &str, unused: Vec<UnusedFeature>) -> NeedsFeed {
    let board = parse_log(text);
    NeedsFeed {
        threads: board.sessions,
        unused,
        events: build_events(text),
        log_at: board.log_at,
    }
}

/// Serve the Needs You feed. Reads the log file on each call.
async fn get_needs_you() -> impl IntoResponse {
    let text = match std::fs::read_to_string(log_path()) {
        Ok(t) => t,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"message": "Watch log is missing. Retry soon."})),
            )
                .into_response();
        }
    };
    let unused = read_unused(&unused_path());
    Json(json!(needs_feed(&text, unused))).into_response()
}

/// Longest owner answer the queue keeps, in chars.
const MAX_ASK_LEN: usize = 2000;

/// One saved owner answer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AskEntry {
    /// Local time the answer landed, like the log lines.
    pub at: String,
    /// Thread the answer is for, when the owner picked one.
    pub thread: Option<String>,
    /// Answer text, redacted on read.
    pub text: String,
}

/// Posted owner answer. Nothing sends to any thread.
#[derive(Debug, Clone, Deserialize)]
pub struct AskPost {
    /// Thread the answer is for, when the owner picked one.
    pub thread: Option<String>,
    /// Answer text.
    pub text: String,
}

/// Queue file path. Env wins, else the factory default.
fn queue_path() -> PathBuf {
    if let Ok(p) = std::env::var("ASK_QUEUE_FILE") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("hearth/factory/ask-queue.jsonl")
}

/// Reject empty or huge answers. Pure, so tests cover it.
fn check_ask(text: &str) -> Result<String, &'static str> {
    let clean = text.trim();
    if clean.is_empty() {
        return Err("Write an answer first.");
    }
    if clean.chars().count() > MAX_ASK_LEN {
        return Err("Answer is too long.");
    }
    Ok(clean.to_string())
}

/// Append one answer to the queue file. Returns the stamp.
/// Creates parent dirs, so a fresh box just works.
fn append_ask(path: &Path, thread: Option<String>, text: String) -> std::io::Result<String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let at = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    let entry = AskEntry {
        at: at.clone(),
        thread,
        text,
    };
    let mut line = serde_json::to_string(&entry).unwrap_or_default();
    line.push('\n');
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    f.write_all(line.as_bytes())?;
    Ok(at)
}

/// Read saved answers. A missing file means none yet, not an error.
/// Broken lines drop out; the queue keeps serving the rest.
fn read_ask(path: &Path) -> Vec<AskEntry> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|l| serde_json::from_str::<AskEntry>(l).ok())
        .map(|mut e| {
            e.text = redact(&e.text);
            e.thread = e.thread.map(|t| redact(&t));
            e
        })
        .collect()
}

/// Serve the saved answers.
async fn get_ask() -> impl IntoResponse {
    Json(json!({ "entries": read_ask(&queue_path()) })).into_response()
}

/// Save one owner answer. It waits in the queue file; nothing
/// sends it to any thread. Sending waits for the owner confirm step.
async fn post_ask(Json(post): Json<AskPost>) -> impl IntoResponse {
    let text = match check_ask(&post.text) {
        Ok(t) => t,
        Err(m) => {
            return (StatusCode::BAD_REQUEST, Json(json!({"message": m}))).into_response();
        }
    };
    let thread = post
        .thread
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    match append_ask(&queue_path(), thread, text) {
        Ok(at) => (StatusCode::OK, Json(json!({"saved": true, "at": at}))).into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"message": "Queue file is down. Retry soon."})),
        )
            .into_response(),
    }
}

/// One alert token into plain words. Unknown tokens pass through.
fn plain_alert(token: &str) -> String {
    if token.starts_with("BLOCKED") {
        "Waiting on approval".to_string()
    } else if token == "GONE" {
        "Session gone".to_string()
    } else if token == "IDLE>30min" {
        "Idle over 30 min".to_string()
    } else if token == "SAME_SCREEN_WHILE_WORKING" {
        "Screen unchanged while working".to_string()
    } else {
        token.to_string()
    }
}

/// Posted feature mark kind.
#[derive(Debug, Clone, Deserialize)]
pub struct FeatureMarkPost {
    /// Mark kind: "used" or "tested_ok".
    pub kind: String,
}

/// Posted owner confirm payload. Accepts `status` or `kind`.
#[derive(Debug, Clone, Deserialize)]
pub struct FeatureConfirmPost {
    /// Confirm status: "works" or "broken".
    pub status: Option<String>,
    /// Alias for status: "works" or "broken".
    pub kind: Option<String>,
}

impl FeatureConfirmPost {
    fn value(&self) -> &str {
        self.status
            .as_deref()
            .or(self.kind.as_deref())
            .unwrap_or("")
            .trim()
    }
}

/// Posted feature step toggle payload.
#[derive(Debug, Clone, Deserialize)]
pub struct FeatureStepPost {
    /// Zero-based step index, when provided.
    pub index: Option<usize>,
    /// Step text match, when index is omitted.
    pub text: Option<String>,
    /// Explicit done value; when omitted, toggles current done state.
    pub done: Option<bool>,
}

/// Features ledger path. Env wins, else the default.
fn features_path() -> PathBuf {
    if let Ok(p) = std::env::var("FEATURES_LEDGER") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("hearth/FEATURES.json")
}

/// Read the features ledger as a JSON object.
fn read_features(path: &Path) -> Result<serde_json::Value, &'static str> {
    let text =
        std::fs::read_to_string(path).map_err(|_| "Features ledger is missing. Retry soon.")?;
    let val: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| "Features ledger is unreadable. Retry soon.")?;
    if !val.get("features").map(|f| f.is_array()).unwrap_or(false) {
        return Err("Features ledger has no features list.");
    }
    Ok(val)
}

/// Serve the features ledger. Reads the file on each call.
async fn get_features() -> impl IntoResponse {
    match read_features(&features_path()) {
        Ok(val) => Json(val).into_response(),
        Err(m) => (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"message": m}))).into_response(),
    }
}

/// Mark errors with HTTP status codes.
#[derive(Debug, Clone, PartialEq)]
enum MarkError {
    BadKind(&'static str),
    NotFound(&'static str),
    Io(&'static str),
}

/// Write a ledger JSON value atomically via temp file plus rename.
fn write_ledger_atomic(path: &Path, root: &serde_json::Value) -> Result<(), MarkError> {
    let mut out = serde_json::to_string_pretty(root)
        .map_err(|_| MarkError::Io("Could not format features ledger."))?;
    out.push('\n');
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    use std::sync::atomic::{AtomicU64, Ordering};
    static TMP_N: AtomicU64 = AtomicU64::new(0);
    let n = TMP_N.fetch_add(1, Ordering::SeqCst);
    let tmp = parent.join(format!(".features-tmp-{}-{}.json", std::process::id(), n));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)
            .map_err(|_| MarkError::Io("Could not write temporary ledger."))?;
        f.write_all(out.as_bytes())
            .and_then(|_| f.sync_all())
            .map_err(|_| {
                let _ = std::fs::remove_file(&tmp);
                MarkError::Io("Could not flush temporary ledger.")
            })?;
    }
    std::fs::rename(&tmp, path).map_err(|_| {
        let _ = std::fs::remove_file(&tmp);
        MarkError::Io("Could not replace features ledger.")
    })?;
    Ok(())
}

/// Mark a feature as used or tested_ok on a ledger file.
/// Writes to a temp sibling file and renames atomically.
/// Keeps unknown fields and never deletes rows.
fn mark_feature_in_file(
    path: &Path,
    id: &str,
    kind: &str,
    today: &str,
    now_stamp: &str,
) -> Result<serde_json::Value, MarkError> {
    let field = match kind {
        "used" => "last_used",
        "tested_ok" => "tested",
        _ => return Err(MarkError::BadKind("Kind must be used or tested_ok.")),
    };
    let text = std::fs::read_to_string(path)
        .map_err(|_| MarkError::Io("Features ledger is missing. Retry soon."))?;
    let mut root: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| MarkError::Io("Features ledger is unreadable. Retry soon."))?;
    let mut updated_row: Option<serde_json::Value> = None;
    {
        let rows = root
            .get_mut("features")
            .and_then(|f| f.as_array_mut())
            .ok_or(MarkError::Io("Features ledger has no features list."))?;
        for row in rows.iter_mut() {
            let Some(obj) = row.as_object_mut() else {
                continue;
            };
            if obj.get("id").and_then(|v| v.as_str()) == Some(id) {
                obj.insert(
                    field.to_string(),
                    serde_json::Value::String(today.to_string()),
                );
                updated_row = Some(serde_json::Value::Object(obj.clone()));
                break;
            }
        }
    }
    let Some(feature) = updated_row else {
        return Err(MarkError::NotFound("Feature id was not found."));
    };
    if let Some(root_obj) = root.as_object_mut() {
        root_obj.insert(
            "updated".to_string(),
            serde_json::Value::String(now_stamp.to_string()),
        );
    }
    write_ledger_atomic(path, &root)?;
    Ok(feature)
}

/// Record owner confirmation ("works" or "broken") on a ledger file.
/// Writes owner_confirmed and owner_confirmed_at atomically; when "works"
/// is confirmed, also sets last_used to today and completed_at if empty.
/// Ignores unknown feature ids (returns NotFound without modifying the file).
fn confirm_feature_in_file(
    path: &Path,
    id: &str,
    status: &str,
    today: &str,
    now_stamp: &str,
) -> Result<serde_json::Value, MarkError> {
    let clean = match status {
        "works" | "broken" => status,
        _ => return Err(MarkError::BadKind("Confirm must be works or broken.")),
    };
    let text = std::fs::read_to_string(path)
        .map_err(|_| MarkError::Io("Features ledger is missing. Retry soon."))?;
    let mut root: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| MarkError::Io("Features ledger is unreadable. Retry soon."))?;
    let mut updated_row: Option<serde_json::Value> = None;
    {
        let rows = root
            .get_mut("features")
            .and_then(|f| f.as_array_mut())
            .ok_or(MarkError::Io("Features ledger has no features list."))?;
        for row in rows.iter_mut() {
            let Some(obj) = row.as_object_mut() else {
                continue;
            };
            if obj.get("id").and_then(|v| v.as_str()) == Some(id) {
                obj.insert(
                    "owner_confirmed".to_string(),
                    serde_json::Value::String(clean.to_string()),
                );
                obj.insert(
                    "owner_confirmed_at".to_string(),
                    serde_json::Value::String(today.to_string()),
                );
                if clean == "works" {
                    obj.insert(
                        "last_used".to_string(),
                        serde_json::Value::String(today.to_string()),
                    );
                    let has_completed = obj
                        .get("completed_at")
                        .and_then(|v| v.as_str())
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false);
                    if !has_completed {
                        obj.insert(
                            "completed_at".to_string(),
                            serde_json::Value::String(now_stamp.to_string()),
                        );
                    }
                }
                updated_row = Some(serde_json::Value::Object(obj.clone()));
                break;
            }
        }
    }
    let Some(feature) = updated_row else {
        return Err(MarkError::NotFound("Feature id was not found."));
    };
    if let Some(root_obj) = root.as_object_mut() {
        root_obj.insert(
            "updated".to_string(),
            serde_json::Value::String(now_stamp.to_string()),
        );
    }
    write_ledger_atomic(path, &root)?;
    Ok(feature)
}

/// POST /v1/watch/features/:id/confirm. Records owner works/broken confirmation.
async fn post_feature_confirm(
    AxumPath(id): AxumPath<String>,
    Json(body): Json<FeatureConfirmPost>,
) -> impl IntoResponse {
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let stamp = now.format("%Y-%m-%d %H:%M").to_string();
    let status = body.value();
    match confirm_feature_in_file(&features_path(), id.trim(), status, &today, &stamp) {
        Ok(feature) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "id": id,
                "owner_confirmed": status,
                "owner_confirmed_at": today,
                "feature": feature
            })),
        )
            .into_response(),
        Err(MarkError::BadKind(m)) => {
            (StatusCode::BAD_REQUEST, Json(json!({"message": m}))).into_response()
        }
        Err(MarkError::NotFound(m)) => (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "ignored": true, "message": m})),
        )
            .into_response(),
        Err(MarkError::Io(m)) => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"message": m}))).into_response()
        }
    }
}

/// Toggle an owner-owned step ("owner": "you") on a feature in the ledger file.
/// Writes atomically and records `done_at` when done is true (clears when false).
fn toggle_feature_step_in_file(
    path: &Path,
    id: &str,
    index: Option<usize>,
    text_match: Option<&str>,
    explicit_done: Option<bool>,
    today: &str,
    now_stamp: &str,
) -> Result<serde_json::Value, MarkError> {
    if index.is_none() && text_match.map(|t| t.trim().is_empty()).unwrap_or(true) {
        return Err(MarkError::BadKind("Pass step index or text."));
    }
    let raw = std::fs::read_to_string(path)
        .map_err(|_| MarkError::Io("Features ledger is missing. Retry soon."))?;
    let mut root: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|_| MarkError::Io("Features ledger is unreadable. Retry soon."))?;
    let mut updated_row: Option<serde_json::Value> = None;
    {
        let rows = root
            .get_mut("features")
            .and_then(|f| f.as_array_mut())
            .ok_or(MarkError::Io("Features ledger has no features list."))?;
        let mut found_feature = false;
        for row in rows.iter_mut() {
            let Some(obj) = row.as_object_mut() else {
                continue;
            };
            if obj.get("id").and_then(|v| v.as_str()) != Some(id) {
                continue;
            }
            found_feature = true;
            let steps = obj
                .get_mut("steps")
                .and_then(|s| s.as_array_mut())
                .ok_or(MarkError::NotFound("Feature has no steps list."))?;
            let target = if let Some(idx) = index {
                steps.get_mut(idx)
            } else if let Some(want) = text_match {
                let want = want.trim();
                steps.iter_mut().find(|s| {
                    s.get("text")
                        .and_then(|v| v.as_str())
                        .map(|t| t.trim() == want)
                        .unwrap_or(false)
                })
            } else {
                None
            };
            let Some(step_val) = target else {
                return Err(MarkError::NotFound("Step was not found."));
            };
            let Some(step_obj) = step_val.as_object_mut() else {
                return Err(MarkError::BadKind("Step row is invalid."));
            };
            let owner = step_obj
                .get("owner")
                .and_then(|v| v.as_str())
                .unwrap_or("you");
            if owner != "you" {
                return Err(MarkError::BadKind(
                    "Only steps owned by you can be toggled here.",
                ));
            }
            let cur_done = step_obj
                .get("done")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let next_done = explicit_done.unwrap_or(!cur_done);
            step_obj.insert("done".to_string(), serde_json::Value::Bool(next_done));
            step_obj.insert(
                "done_at".to_string(),
                serde_json::Value::String(if next_done {
                    today.to_string()
                } else {
                    String::new()
                }),
            );
            updated_row = Some(serde_json::Value::Object(obj.clone()));
            break;
        }
        if !found_feature {
            return Err(MarkError::NotFound("Feature id was not found."));
        }
    }
    let Some(feature) = updated_row else {
        return Err(MarkError::NotFound("Feature id was not found."));
    };
    if let Some(root_obj) = root.as_object_mut() {
        root_obj.insert(
            "updated".to_string(),
            serde_json::Value::String(now_stamp.to_string()),
        );
    }
    write_ledger_atomic(path, &root)?;
    Ok(feature)
}

/// POST /v1/watch/features/:id/step. Toggles a step owned by "you".
async fn post_feature_step(
    AxumPath(id): AxumPath<String>,
    Json(body): Json<FeatureStepPost>,
) -> impl IntoResponse {
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let stamp = now.format("%Y-%m-%d %H:%M").to_string();
    match toggle_feature_step_in_file(
        &features_path(),
        id.trim(),
        body.index,
        body.text.as_deref(),
        body.done,
        &today,
        &stamp,
    ) {
        Ok(feature) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "id": id,
                "date": today,
                "feature": feature
            })),
        )
            .into_response(),
        Err(MarkError::BadKind(m)) => {
            (StatusCode::BAD_REQUEST, Json(json!({"message": m}))).into_response()
        }
        Err(MarkError::NotFound(m)) => (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "ignored": true, "message": m})),
        )
            .into_response(),
        Err(MarkError::Io(m)) => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"message": m}))).into_response()
        }
    }
}

/// POST /v1/features/:id/mark. Sets last_used or tested to today.
async fn post_feature_mark(
    AxumPath(id): AxumPath<String>,
    Json(body): Json<FeatureMarkPost>,
) -> impl IntoResponse {
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let stamp = now.format("%Y-%m-%d %H:%M").to_string();
    match mark_feature_in_file(
        &features_path(),
        id.trim(),
        body.kind.trim(),
        &today,
        &stamp,
    ) {
        Ok(feature) => (
            StatusCode::OK,
            Json(
                json!({"ok": true, "id": id, "kind": body.kind, "date": today, "feature": feature}),
            ),
        )
            .into_response(),
        Err(MarkError::BadKind(m)) => {
            (StatusCode::BAD_REQUEST, Json(json!({"message": m}))).into_response()
        }
        Err(MarkError::NotFound(m)) => {
            (StatusCode::NOT_FOUND, Json(json!({"message": m}))).into_response()
        }
        Err(MarkError::Io(m)) => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"message": m}))).into_response()
        }
    }
}

/// Posted screen open payload.
#[derive(Debug, Clone, Deserialize)]
pub struct ScreenOpenPost {
    /// App route, e.g. "/threads".
    pub route: String,
    /// First open stamp.
    pub first_open: String,
    /// Last open stamp.
    pub last_open: String,
}

/// Posted screen confirm payload.
#[derive(Debug, Clone, Deserialize)]
pub struct ScreenConfirmPost {
    /// App route, e.g. "/threads".
    pub route: String,
    /// Owner verdict: works or broken.
    pub confirmed: String,
}

/// Validate a screen route. App routes start with a slash.
fn valid_screen_route(route: &str) -> bool {
    let r = route.trim();
    !r.is_empty() && r.starts_with('/') && r.len() <= 200 && !r.contains([' ', '\t', '\n'])
}

/// Validate an open stamp. Non-empty short text only.
fn valid_stamp(stamp: &str) -> bool {
    let s = stamp.trim();
    !s.is_empty() && s.len() <= 32
}

/// Merge a screen open into the ledger file. First stays earliest,
/// last moves forward. Creates the screens map when missing.
fn record_screen_open_in_file(
    path: &Path,
    route: &str,
    first: &str,
    last: &str,
) -> Result<serde_json::Value, MarkError> {
    let route = route.trim();
    if !valid_screen_route(route) {
        return Err(MarkError::BadKind("Name a screen route like /threads."));
    }
    if !valid_stamp(first) || !valid_stamp(last) {
        return Err(MarkError::BadKind("Pass first and last open times."));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|_| MarkError::Io("Features ledger is missing. Retry soon."))?;
    let mut root: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| MarkError::Io("Features ledger is unreadable. Retry soon."))?;
    let entry = {
        let obj = root
            .as_object_mut()
            .ok_or(MarkError::Io("Features ledger has no root object."))?;
        let map = obj
            .entry("screens")
            .or_insert_with(|| serde_json::Value::Object(Default::default()));
        let map = map
            .as_object_mut()
            .ok_or(MarkError::Io("Features ledger screens block is broken."))?;
        let slot = map.entry(route).or_insert_with(|| serde_json::json!({}));
        let row = slot
            .as_object_mut()
            .ok_or(MarkError::Io("Features ledger screen row is broken."))?;
        let old_first = row.get("first_open").and_then(|v| v.as_str());
        let old_last = row.get("last_open").and_then(|v| v.as_str());
        let first_now = match old_first {
            Some(o) if o <= first.trim() => o.to_string(),
            _ => first.trim().to_string(),
        };
        let last_now = match old_last {
            Some(o) if o >= last.trim() => o.to_string(),
            _ => last.trim().to_string(),
        };
        row.insert(
            "first_open".to_string(),
            serde_json::Value::String(first_now.clone()),
        );
        row.insert(
            "last_open".to_string(),
            serde_json::Value::String(last_now.clone()),
        );
        serde_json::json!({"route": route, "first_open": first_now, "last_open": last_now})
    };
    write_ledger_atomic(path, &root)?;
    Ok(entry)
}

/// Record an owner works/broken confirmation for a screen.
fn confirm_screen_in_file(
    path: &Path,
    route: &str,
    confirmed: &str,
    today: &str,
) -> Result<serde_json::Value, MarkError> {
    let route = route.trim();
    if !valid_screen_route(route) {
        return Err(MarkError::BadKind("Name a screen route like /threads."));
    }
    let status = confirmed.trim().to_lowercase();
    if status != "works" && status != "broken" {
        return Err(MarkError::BadKind("Confirmed must be works or broken."));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|_| MarkError::Io("Features ledger is missing. Retry soon."))?;
    let mut root: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| MarkError::Io("Features ledger is unreadable. Retry soon."))?;
    let entry = {
        let obj = root
            .as_object_mut()
            .ok_or(MarkError::Io("Features ledger has no root object."))?;
        let map = obj
            .entry("screens")
            .or_insert_with(|| serde_json::Value::Object(Default::default()));
        let map = map
            .as_object_mut()
            .ok_or(MarkError::Io("Features ledger screens block is broken."))?;
        let slot = map.entry(route).or_insert_with(|| serde_json::json!({}));
        let row = slot
            .as_object_mut()
            .ok_or(MarkError::Io("Features ledger screen row is broken."))?;
        row.insert(
            "confirmed".to_string(),
            serde_json::Value::String(status.clone()),
        );
        row.insert(
            "confirmed_at".to_string(),
            serde_json::Value::String(today.to_string()),
        );
        serde_json::json!({"route": route, "confirmed": status, "confirmed_at": today})
    };
    write_ledger_atomic(path, &root)?;
    Ok(entry)
}

/// Read the screens map from the ledger file. Missing means empty.
fn read_screens(path: &Path) -> Result<serde_json::Value, MarkError> {
    let text = std::fs::read_to_string(path)
        .map_err(|_| MarkError::Io("Features ledger is missing. Retry soon."))?;
    let root: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| MarkError::Io("Features ledger is unreadable. Retry soon."))?;
    Ok(root
        .get("screens")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(Default::default())))
}

/// Serve the screens map. Reads the file on each call.
async fn get_screens() -> impl IntoResponse {
    match read_screens(&features_path()) {
        Ok(val) => Json(json!({"screens": val})).into_response(),
        Err(MarkError::Io(m)) => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"message": m}))).into_response()
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"message": "Screens block is broken. Retry soon."})),
        )
            .into_response(),
    }
}

/// POST /v1/watch/screens/open. Merges first and last open stamps.
async fn post_screen_open(Json(body): Json<ScreenOpenPost>) -> impl IntoResponse {
    match record_screen_open_in_file(
        &features_path(),
        body.route.trim(),
        body.first_open.trim(),
        body.last_open.trim(),
    ) {
        Ok(entry) => (StatusCode::OK, Json(json!({"ok": true, "screen": entry}))).into_response(),
        Err(MarkError::BadKind(m)) => {
            (StatusCode::BAD_REQUEST, Json(json!({"message": m}))).into_response()
        }
        Err(MarkError::NotFound(m)) => {
            (StatusCode::NOT_FOUND, Json(json!({"message": m}))).into_response()
        }
        Err(MarkError::Io(m)) => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"message": m}))).into_response()
        }
    }
}

/// One lane status row from a HISTORY-STATUS mirror file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct LaneStatus {
    lane: String,
    at: String,
    note: String,
    rows: Option<u64>,
    errors: Option<u64>,
    pct: Option<f64>,
    spend: String,
}

/// Parse one mirror line. Unknown segments join the note, never drop.
fn parse_lane_line(line: &str) -> Option<LaneStatus> {
    let body = line.trim().strip_prefix('-').unwrap_or(line.trim()).trim();
    let mut segs = body.split('|').map(str::trim);
    let at = segs.next().unwrap_or("").to_string();
    let mut lane: Option<String> = None;
    let mut note_parts: Vec<&str> = vec![];
    let mut rows: Option<u64> = None;
    let mut errors: Option<u64> = None;
    let mut pct: Option<f64> = None;
    let mut spend = String::new();
    for s in segs {
        if let Some(v) = s.strip_prefix("lane=") {
            lane = Some(v.trim().to_string());
        } else if let Some(v) = s.strip_prefix("rows=") {
            rows = v.trim().parse().ok();
        } else if let Some(v) = s.strip_prefix("errors=") {
            errors = v.trim().parse().ok();
        } else if let Some(v) = s.strip_prefix("pct=") {
            pct = v.trim().trim_end_matches('%').parse().ok();
        } else if s.starts_with("est spend") {
            spend = s
                .split(':')
                .skip(1)
                .collect::<Vec<_>>()
                .join(":")
                .trim()
                .to_string();
        } else if !s.is_empty() {
            note_parts.push(s);
        }
    }
    Some(LaneStatus {
        lane: lane?,
        at,
        note: note_parts.join(" | ").chars().take(300).collect(),
        rows,
        errors,
        pct,
        spend: spend.chars().take(160).collect(),
    })
}

/// Latest lane line per mirror file. A missing dir reads as empty.
fn read_lane_dir(dir: &Path) -> Vec<LaneStatus> {
    let mut out: Vec<LaneStatus> = vec![];
    let rd = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return out,
    };
    for ent in rd.flatten() {
        let name = ent.file_name().to_string_lossy().into_owned();
        if !name.starts_with("HISTORY-STATUS-") || !name.ends_with(".md") {
            continue;
        }
        let text = match std::fs::read_to_string(ent.path()) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if let Some(line) = text.lines().rev().find(|l| l.contains("lane=")) {
            if let Some(st) = parse_lane_line(line) {
                out.push(st);
            }
        }
    }
    out.sort_by(|a, b| a.lane.cmp(&b.lane));
    out
}

/// One repo commit for the Dev timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct DevCommit {
    sha: String,
    msg: String,
    at: String,
}

/// Read-only git log. Fixed args only. No caller input reaches git.
fn read_git_log(repo: &Path) -> Vec<DevCommit> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["log", "--format=%H%x1f%s%x1f%cI", "-15"])
        .output();
    let out = match out {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(parse_git_line)
        .collect()
}

fn parse_git_line(line: &str) -> Option<DevCommit> {
    let mut p = line.split('\u{1f}');
    let sha = p.next()?.trim();
    let msg = p.next().unwrap_or("").trim();
    let at = p.next().unwrap_or("").trim();
    if sha.is_empty() {
        return None;
    }
    Some(DevCommit {
        sha: sha.chars().take(40).collect(),
        msg: msg.chars().take(120).collect(),
        at: at.to_string(),
    })
}

/// Newest test run summary from a nextest JUnit file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct DevTests {
    tests: u64,
    failures: u64,
    errors: u64,
    at: String,
}

/// Newest junit.xml under repo/target. Four levels deep, 200 files max.
fn read_test_summary(repo: &Path) -> Option<DevTests> {
    let target = repo.join("target");
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut seen = 0;
    let mut stack: Vec<(PathBuf, u8)> = vec![(target, 0)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > 4 {
            continue;
        }
        let rd = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for ent in rd.flatten() {
            seen += 1;
            if seen > 200 {
                break;
            }
            let p = ent.path();
            if p.is_dir() {
                stack.push((p, depth + 1));
            } else if p.file_name().and_then(|n| n.to_str()) == Some("junit.xml") {
                let mt = ent.metadata().and_then(|m| m.modified()).ok();
                if let Some(mt) = mt {
                    let newer = best.as_ref().is_none_or(|(_, b)| mt > *b);
                    if newer {
                        best = Some((p, mt));
                    }
                }
            }
        }
    }
    let (path, _) = best?;
    let text = std::fs::read_to_string(&path).ok()?;
    parse_junit_attrs(&text)
}

/// Read counts off the first testsuite tag. Lenient, never panics.
fn parse_junit_attrs(text: &str) -> Option<DevTests> {
    let start = text.find("<testsuite")?;
    let tag: String = text[start..].chars().take(600).collect();
    let end = tag.find('>')?;
    let tag = &tag[..end];
    let attr = |name: &str| -> Option<String> {
        let key = format!("{name}=\"");
        let i = tag.find(&key)? + key.len();
        let j = tag[i..].find('"')?;
        Some(tag[i..i + j].to_string())
    };
    Some(DevTests {
        tests: attr("tests").and_then(|v| v.parse().ok()).unwrap_or(0),
        failures: attr("failures").and_then(|v| v.parse().ok()).unwrap_or(0),
        errors: attr("errors").and_then(|v| v.parse().ok()).unwrap_or(0),
        at: attr("timestamp").unwrap_or_default(),
    })
}

/// GET /v1/watch/dev/feed. Read-only lane status, commits, tests.
async fn get_dev_feed() -> impl IntoResponse {
    let repo = std::env::var("LIFEOS_REPO").unwrap_or_else(|_| ".".to_string());
    let repo = PathBuf::from(repo);
    let lanes = match std::env::var("DEV_STATUS_DIR") {
        Ok(d) if !d.trim().is_empty() => read_lane_dir(Path::new(&d)),
        _ => vec![],
    };
    let lanes_note = if lanes.is_empty() {
        "Lane status is not on record."
    } else {
        ""
    };
    let commits = read_git_log(&repo);
    let commits_note = if commits.is_empty() {
        "Repo is not reachable."
    } else {
        ""
    };
    let tests = read_test_summary(&repo);
    let tests_note = if tests.is_none() {
        "No test runs on record."
    } else {
        ""
    };
    Json(json!({
        "lanes": lanes,
        "lanes_note": lanes_note,
        "commits": commits,
        "commits_note": commits_note,
        "tests": tests,
        "tests_note": tests_note,
    }))
    .into_response()
}

/// POST /v1/watch/screens/confirm. Records owner works/broken verdict.
async fn post_screen_confirm(Json(body): Json<ScreenConfirmPost>) -> impl IntoResponse {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    match confirm_screen_in_file(
        &features_path(),
        body.route.trim(),
        body.confirmed.trim(),
        &today,
    ) {
        Ok(entry) => (StatusCode::OK, Json(json!({"ok": true, "screen": entry}))).into_response(),
        Err(MarkError::BadKind(m)) => {
            (StatusCode::BAD_REQUEST, Json(json!({"message": m}))).into_response()
        }
        Err(MarkError::NotFound(m)) => {
            (StatusCode::NOT_FOUND, Json(json!({"message": m}))).into_response()
        }
        Err(MarkError::Io(m)) => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"message": m}))).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "- 2026-11-01 01:32 | w1:p19 blocked BLOCKED(approval?) | w1:p1X done\n\
         - 2026-11-01 01:37 | w1:p19 blocked BLOCKED(approval?) IDLE>30min | w1:p1X done\n\
         - 2026-11-01 01:42 | w1:p19 done | w1:p1X done\n";

    #[test]
    fn parses_latest_state_and_plain_alerts() {
        let b = parse_log(SAMPLE);
        assert_eq!(b.log_at.as_deref(), Some("2026-11-01 01:42"));
        let lifeos = &b.sessions[0];
        assert_eq!(lifeos.pane, "w1:p19");
        assert_eq!(lifeos.name, "w1:p19");
        assert_eq!(lifeos.state, "done");
        assert!(lifeos.alerts.is_empty());
        assert_eq!(lifeos.changed_at.as_deref(), Some("2026-11-01 01:42"));
    }

    #[test]
    fn changed_since_finds_run_start() {
        let b = parse_log(SAMPLE);
        let trade = &b.sessions[1];
        assert_eq!(trade.state, "done");
        assert_eq!(trade.changed_at.as_deref(), Some("2026-11-01 01:32"));
    }

    #[test]
    fn maps_each_alert_to_plain_words() {
        let one = "- 2026-11-01 01:37 | w1:p21 blocked BLOCKED(approval?) IDLE>30min SAME_SCREEN_WHILE_WORKING\n";
        let b = parse_log(one);
        let station = b.sessions.iter().find(|s| s.pane == "w1:p21").unwrap();
        assert_eq!(
            station.alerts,
            vec![
                "Waiting on approval",
                "Idle over 30 min",
                "Screen unchanged while working"
            ]
        );
    }

    #[test]
    fn names_come_from_new_lines_old_lines_fall_back() {
        let b = parse_log(
            "- 2026-11-01 01:00 | w1:p19 lifeos working | w1:p99 done idle | w1:p1X done\n",
        );
        let by_pane = |p: &str| {
            b.sessions
                .iter()
                .find(|s| s.pane == p)
                .unwrap()
                .name
                .clone()
        };
        assert_eq!(by_pane("w1:p19"), "lifeos");
        assert_eq!(by_pane("w1:p99"), "done");
        assert_eq!(by_pane("w1:p1X"), "w1:p1X");
        let panes: Vec<&str> = b.sessions.iter().map(|s| s.pane.as_str()).collect();
        assert_eq!(panes, vec!["w1:p19", "w1:p1X", "w1:p99"]);
    }

    #[test]
    fn gone_runs_over_an_hour_drop_out() {
        let b = parse_log(
            "- 2026-11-01 01:00 | w1:p19 working\n\
             - 2026-11-01 01:05 | w1:p19 gone GONE\n\
             - 2026-11-01 02:10 | w1:p19 gone GONE\n\
             - 2026-11-01 02:15 | w1:p1X done | w1:p19 gone GONE\n",
        );
        assert!(b.sessions.iter().all(|s| s.pane != "w1:p19"));
        assert_eq!(b.sessions.len(), 1);
    }

    #[test]
    fn fresh_gone_shows_red_until_the_hour_passes() {
        let b = parse_log(
            "- 2026-11-01 02:00 | w1:p19 working\n\
             - 2026-11-01 02:05 | w1:p19 gone GONE\n\
             - 2026-11-01 02:15 | w1:p19 gone GONE\n",
        );
        let gone = b.sessions.iter().find(|s| s.pane == "w1:p19").unwrap();
        assert_eq!(gone.state, "gone");
        let t = thread_board(&b);
        assert_eq!(t.threads[0].flow, "paused");
    }

    #[test]
    fn panes_unseen_over_an_hour_drop_out() {
        let b = parse_log(
            "- 2026-11-01 01:00 | w1:p19 done\n\
             - 2026-11-01 03:00 | w1:p1X done\n",
        );
        assert!(b.sessions.iter().all(|s| s.pane != "w1:p19"));
        assert_eq!(b.sessions.len(), 1);
    }

    #[test]
    fn stamp_math_crosses_midnight() {
        let a = stamp_min("2026-11-01 23:55").unwrap();
        let b = stamp_min("2026-11-02 00:05").unwrap();
        assert_eq!(b - a, 10);
        assert!(stamp_min("not a stamp").is_none());
    }

    #[test]
    fn live_log_parses_known_states() {
        let path = super::log_path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };
        let b = parse_log(&text);
        assert!(!b.sessions.is_empty());
        assert!(b.log_at.is_some());
        for s in &b.sessions {
            assert!(["working", "idle", "blocked", "done", "gone", "unknown"]
                .contains(&s.state.as_str()));
        }
    }

    #[test]
    fn redact_masks_token_runs_and_keeps_prose() {
        assert_eq!(
            redact("key sk-fake-AAAABBBBCCCCDDDDEEEE here"),
            "key [redacted] here"
        );
        assert_eq!(redact("short code abc123 stays"), "short code abc123 stays");
        // Long hostnames mask too. Fail closed: a masked host is
        // noise, a leaked key is a breach.
        assert_eq!(
            redact("see https://very-long-hostname-01.example.ts.net:8447/v1 soon"),
            "see https://[redacted]:8447/v1 soon"
        );
        assert_eq!(
            redact("path /home/user/hearth/web/src-tauri/readme ok"),
            "path /home/user/hearth/web/src-tauri/readme ok"
        );
    }

    #[test]
    fn junk_lines_are_ignored() {
        let b = parse_log("# title\n\nnot a line\n- 2026-11-01 01:42 | w1:p22 done\n");
        assert_eq!(b.sessions.len(), 1);
        assert_eq!(b.sessions[0].pane, "w1:p22");
        assert_eq!(b.log_at.as_deref(), Some("2026-11-01 01:42"));
    }

    const FLOWS: &str = "- 2026-11-01 12:00 | w1:p19 working | w1:p1X blocked BLOCKED(approval?) | w1:p1Z idle IDLE>30min | w1:p21 done\n";

    #[test]
    fn blocked_with_approval_sets_question() {
        let b = parse_log(FLOWS);
        let trade = b.sessions.iter().find(|s| s.pane == "w1:p1X").unwrap();
        assert!(trade.question.is_some());
        assert!(trade.question.as_deref().unwrap().contains("approval"));
        let lifeos = &b.sessions[0];
        assert!(lifeos.question.is_none());
    }

    #[test]
    fn flows_map_idle_to_calm_waiting() {
        let t = thread_board(&parse_log(FLOWS));
        assert_eq!(t.threads.len(), 4);
        let flow = |p: &str| t.threads.iter().find(|s| s.pane == p).unwrap().flow.clone();
        assert_eq!(flow("w1:p19"), "building");
        assert_eq!(flow("w1:p1X"), "need");
        assert_eq!(flow("w1:p1Z"), "waiting");
        assert_eq!(flow("w1:p21"), "review");
    }

    #[test]
    fn needs_answer_queue_holds_only_waiters() {
        let t = thread_board(&parse_log(FLOWS));
        assert_eq!(t.needs_answer.len(), 1);
        assert_eq!(t.needs_answer[0].thread, "w1:p1X");
        assert!(t.needs_answer[0].recommendation.is_none());
    }

    #[test]
    fn lines_speak_plain_words() {
        let t = thread_board(&parse_log(FLOWS));
        let line = |p: &str| t.threads.iter().find(|s| s.pane == p).unwrap().line.clone();
        assert_eq!(line("w1:p19"), "Builds in the background.");
        assert_eq!(line("w1:p1X"), "Waits on the owner answer.");
        assert_eq!(line("w1:p1Z"), "Waits for the next task.");
        assert_eq!(line("w1:p21"), "Ready for review.");
    }

    #[test]
    fn live_log_boards_all_flows() {
        let path = super::log_path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };
        let t = thread_board(&parse_log(&text));
        assert!(!t.threads.is_empty());
        for s in &t.threads {
            assert!(["need", "building", "paused", "review", "waiting"].contains(&s.flow.as_str()));
            assert!(!s.line.is_empty());
        }
        assert!(t.needs_answer.len() <= t.threads.len());
    }

    #[test]
    fn check_ask_rejects_empty_and_huge() {
        assert!(check_ask("   ").is_err());
        assert!(check_ask("go").is_ok());
        assert!(check_ask(&"x".repeat(2001)).is_err());
        assert!(check_ask(&"x".repeat(2000)).is_ok());
    }

    /// Fresh queue path per test. Parallel-safe: pid plus counter.
    fn fresh_queue() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("askq-{}-{}.jsonl", std::process::id(), n))
    }

    #[test]
    fn ask_roundtrip_appends_and_reads() {
        let p = fresh_queue();
        let at = append_ask(&p, Some("lifeos".to_string()), "Ship it.".to_string()).unwrap();
        assert_eq!(at.len(), 16);
        append_ask(&p, None, "Later.".to_string()).unwrap();
        let back = read_ask(&p);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].thread.as_deref(), Some("lifeos"));
        assert_eq!(back[0].text, "Ship it.");
        assert!(back[1].thread.is_none());
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn ask_read_missing_file_means_empty() {
        let back = read_ask(&fresh_queue());
        assert!(back.is_empty());
    }

    #[test]
    fn ask_read_skips_broken_lines() {
        let p = fresh_queue();
        std::fs::write(&p, "junk\n{\"at\":\"t\",\"thread\":null,\"text\":\"ok\"}\n").unwrap();
        let back = read_ask(&p);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].text, "ok");
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn events_fire_on_change_only_newest_first() {
        let ev = build_events(SAMPLE);
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0].at, "2026-11-01 01:42");
        assert_eq!(ev[0].tag, "DONE");
        assert_eq!(ev[0].text, "w1:p19 finishes. Ready for review.");
        assert_eq!(ev[1].tag, "NEED");
        assert_eq!(ev[1].text, "w1:p19 waits on approval.");
    }

    #[test]
    fn events_cap_at_eight() {
        let mut text = String::new();
        for i in 0..10 {
            let st = if i % 2 == 0 { "working" } else { "idle" };
            text.push_str(&format!("- 2026-11-01 0{i}:00 | w1:p19 {st}\n"));
        }
        let ev = build_events(&text);
        assert_eq!(ev.len(), 8);
        assert_eq!(ev[0].at, "2026-11-01 09:00");
    }

    #[test]
    fn unused_missing_file_means_empty() {
        let back = read_unused(&fresh_queue());
        assert!(back.is_empty());
    }

    #[test]
    fn unused_roundtrip_reads_rows() {
        let p = fresh_queue();
        std::fs::write(
            &p,
            r#"{"features": [{"id": "u1", "name": "Fake build", "route": "/builds", "shipped_at": "2026-10-30", "note": "Sample row."}]}"#,
        )
        .unwrap();
        let back = read_unused(&p);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].name, "Fake build");
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn needs_feed_carries_threads_events_and_log_time() {
        let feed = needs_feed(SAMPLE, Vec::new());
        assert_eq!(feed.threads.len(), 2);
        assert_eq!(feed.events.len(), 2);
        assert_eq!(feed.log_at.as_deref(), Some("2026-11-01 01:42"));
        assert!(feed.unused.is_empty());
    }

    const SAMPLE_FEATURES: &str = r#"{
  "_how": "Feature ledger.",
  "updated": "2026-11-01 20:00",
  "extra_top": "keep-me",
  "features": [
    {
      "id": "f1",
      "name": "Alpha",
      "area": "LifeOS app",
      "stage": "deployed",
      "where": "https://example.test",
      "tested": "not tested",
      "deployed": "APK 1003",
      "last_used": "never",
      "next": "try it",
      "custom_field": 42
    },
    {
      "id": "f2",
      "name": "Beta",
      "area": "Tank",
      "stage": "built",
      "where": "branch",
      "tested": "unit tests",
      "deployed": "not deployed",
      "last_used": "never",
      "next": "ship"
    }
  ]
}
"#;

    #[test]
    fn feature_mark_used_and_tested_ok_atomic_preserves_unknown_fields_and_rows() {
        let p = fresh_queue();
        std::fs::write(&p, SAMPLE_FEATURES).unwrap();

        let row = mark_feature_in_file(&p, "f1", "used", "2026-11-01", "2026-11-01 23:55").unwrap();
        assert_eq!(
            row.get("last_used").and_then(|v| v.as_str()),
            Some("2026-11-01")
        );
        assert_eq!(row.get("custom_field").and_then(|v| v.as_i64()), Some(42));

        let row2 =
            mark_feature_in_file(&p, "f2", "tested_ok", "2026-11-01", "2026-11-01 23:56").unwrap();
        assert_eq!(
            row2.get("tested").and_then(|v| v.as_str()),
            Some("2026-11-01")
        );

        let read_back = read_features(&p).unwrap();
        assert_eq!(
            read_back.get("extra_top").and_then(|v| v.as_str()),
            Some("keep-me")
        );
        assert_eq!(
            read_back.get("updated").and_then(|v| v.as_str()),
            Some("2026-11-01 23:56")
        );
        let rows = read_back
            .get("features")
            .and_then(|f| f.as_array())
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].get("last_used").and_then(|v| v.as_str()),
            Some("2026-11-01")
        );
        assert_eq!(
            rows[0].get("custom_field").and_then(|v| v.as_i64()),
            Some(42)
        );
        assert_eq!(
            rows[1].get("tested").and_then(|v| v.as_str()),
            Some("2026-11-01")
        );
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn feature_mark_validates_id_and_kind() {
        let p = fresh_queue();
        std::fs::write(&p, SAMPLE_FEATURES).unwrap();

        assert_eq!(
            mark_feature_in_file(&p, "missing-id", "used", "2026-11-01", "2026-11-01 23:55"),
            Err(MarkError::NotFound("Feature id was not found."))
        );
        assert_eq!(
            mark_feature_in_file(&p, "f1", "bogus", "2026-11-01", "2026-11-01 23:55"),
            Err(MarkError::BadKind("Kind must be used or tested_ok."))
        );
        let read_back = read_features(&p).unwrap();
        let rows = read_back
            .get("features")
            .and_then(|f| f.as_array())
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].get("last_used").and_then(|v| v.as_str()),
            Some("never")
        );
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn feature_confirm_works_and_broken_atomic_ignores_unknown_id() {
        let p = fresh_queue();
        std::fs::write(&p, SAMPLE_FEATURES).unwrap();

        let row =
            confirm_feature_in_file(&p, "f1", "works", "2026-11-02", "2026-11-02 06:20").unwrap();
        assert_eq!(
            row.get("owner_confirmed").and_then(|v| v.as_str()),
            Some("works")
        );
        assert_eq!(
            row.get("owner_confirmed_at").and_then(|v| v.as_str()),
            Some("2026-11-02")
        );
        assert_eq!(
            row.get("last_used").and_then(|v| v.as_str()),
            Some("2026-11-02")
        );
        assert_eq!(
            row.get("completed_at").and_then(|v| v.as_str()),
            Some("2026-11-02 06:20")
        );

        let row2 =
            confirm_feature_in_file(&p, "f2", "broken", "2026-11-02", "2026-11-02 06:21").unwrap();
        assert_eq!(
            row2.get("owner_confirmed").and_then(|v| v.as_str()),
            Some("broken")
        );
        assert_eq!(
            row2.get("owner_confirmed_at").and_then(|v| v.as_str()),
            Some("2026-11-02")
        );

        // Unknown id returns NotFound and leaves ledger untouched.
        assert_eq!(
            confirm_feature_in_file(&p, "unknown-id", "works", "2026-11-02", "2026-11-02 06:22"),
            Err(MarkError::NotFound("Feature id was not found."))
        );
        assert_eq!(
            confirm_feature_in_file(&p, "f1", "maybe", "2026-11-02", "2026-11-02 06:22"),
            Err(MarkError::BadKind("Confirm must be works or broken."))
        );

        let read_back = read_features(&p).unwrap();
        let rows = read_back
            .get("features")
            .and_then(|f| f.as_array())
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].get("owner_confirmed").and_then(|v| v.as_str()),
            Some("works")
        );
        assert_eq!(
            rows[1].get("owner_confirmed").and_then(|v| v.as_str()),
            Some("broken")
        );
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn feature_step_toggles_you_step_records_date_and_rejects_agent_step() {
        let p = fresh_queue();
        let with_steps = r#"{
  "updated": "2026-11-02 06:00",
  "features": [
    {
      "id": "f1",
      "name": "Alpha",
      "area": "LifeOS app",
      "stage": "deployed",
      "where": "https://example.test",
      "tested": "ok",
      "deployed": "APK 1004",
      "last_used": "2026-11-02",
      "next": "confirm",
      "steps": [
        { "text": "tap Screen", "owner": "you", "done": false, "done_at": "" },
        { "text": "restart service", "owner": "agent", "done": false, "done_at": "" }
      ]
    }
  ]
}
"#;
        std::fs::write(&p, with_steps).unwrap();

        let row = toggle_feature_step_in_file(
            &p,
            "f1",
            Some(0),
            None,
            None,
            "2026-11-02",
            "2026-11-02 06:25",
        )
        .unwrap();
        let steps = row.get("steps").and_then(|v| v.as_array()).unwrap();
        assert_eq!(steps[0].get("done").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            steps[0].get("done_at").and_then(|v| v.as_str()),
            Some("2026-11-02")
        );

        // Toggle back off by text match.
        let row2 = toggle_feature_step_in_file(
            &p,
            "f1",
            None,
            Some("tap Screen"),
            None,
            "2026-11-02",
            "2026-11-02 06:26",
        )
        .unwrap();
        let steps2 = row2.get("steps").and_then(|v| v.as_array()).unwrap();
        assert_eq!(steps2[0].get("done").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(steps2[0].get("done_at").and_then(|v| v.as_str()), Some(""));

        // Agent-owned step cannot be toggled by the owner endpoint.
        assert_eq!(
            toggle_feature_step_in_file(
                &p,
                "f1",
                Some(1),
                None,
                None,
                "2026-11-02",
                "2026-11-02 06:27"
            ),
            Err(MarkError::BadKind(
                "Only steps owned by you can be toggled here."
            ))
        );

        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn screen_open_creates_and_merges_first_last() {
        let p = fresh_queue();
        std::fs::write(&p, SAMPLE_FEATURES).unwrap();
        let e = record_screen_open_in_file(&p, "/threads", "2026-11-02 08:00", "2026-11-02 08:00")
            .unwrap();
        assert_eq!(e["first_open"], "2026-11-02 08:00");
        assert_eq!(e["last_open"], "2026-11-02 08:00");
        let e2 = record_screen_open_in_file(&p, "/threads", "2026-11-02 09:00", "2026-11-02 09:00")
            .unwrap();
        assert_eq!(e2["first_open"], "2026-11-02 08:00");
        assert_eq!(e2["last_open"], "2026-11-02 09:00");
        let back = read_features(&p).unwrap();
        assert_eq!(back["features"].as_array().unwrap().len(), 2);
        assert_eq!(back["screens"]["/threads"]["last_open"], "2026-11-02 09:00");
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn screen_open_rejects_bad_routes_and_stamps() {
        let p = fresh_queue();
        std::fs::write(&p, SAMPLE_FEATURES).unwrap();
        assert!(record_screen_open_in_file(&p, "", "a", "b").is_err());
        assert!(record_screen_open_in_file(&p, "threads", "a", "b").is_err());
        assert!(record_screen_open_in_file(&p, "/a b", "a", "b").is_err());
        assert!(record_screen_open_in_file(&p, "/threads", "", "b").is_err());
        assert!(record_screen_open_in_file(&p, "/threads", "a", "").is_err());
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn screen_confirm_sets_status_and_date() {
        let p = fresh_queue();
        std::fs::write(&p, SAMPLE_FEATURES).unwrap();
        let e = confirm_screen_in_file(&p, "/threads", "Works", "2026-11-02").unwrap();
        assert_eq!(e["confirmed"], "works");
        assert_eq!(e["confirmed_at"], "2026-11-02");
        assert!(confirm_screen_in_file(&p, "/threads", "maybe", "2026-11-02").is_err());
        assert!(confirm_screen_in_file(&p, "", "works", "2026-11-02").is_err());
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn screens_read_returns_empty_map_when_missing() {
        let p = fresh_queue();
        std::fs::write(&p, SAMPLE_FEATURES).unwrap();
        let v = read_screens(&p).unwrap();
        assert_eq!(v, serde_json::json!({}));
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn lane_line_parses_mirror_format() {
        let st = parse_lane_line("- 2026-11-02T07:15Z | lane=earnings | tier1=no (scaffold ok) | rows=0 | pct=5% | errors=0 | est spend: unmeasurable, tool calls ~22/150").unwrap();
        assert_eq!(st.lane, "earnings");
        assert_eq!(st.at, "2026-11-02T07:15Z");
        assert_eq!(st.note, "tier1=no (scaffold ok)");
        assert_eq!(st.rows, Some(0));
        assert_eq!(st.pct, Some(5.0));
        assert_eq!(st.errors, Some(0));
        assert_eq!(st.spend, "unmeasurable, tool calls ~22/150");
    }

    #[test]
    fn lane_line_needs_lane_and_tolerates_gaps() {
        assert!(parse_lane_line("- 2026-11-02 | rows=3").is_none());
        let st = parse_lane_line("- 2026-11-02 | lane=solo").unwrap();
        assert_eq!(st.lane, "solo");
        assert_eq!(st.rows, None);
        assert_eq!(st.note, "");
    }

    #[test]
    fn lane_dir_reads_latest_line_per_file() {
        let dir = std::env::temp_dir().join(format!("lanes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("HISTORY-STATUS-a.md"),
            "# mirror\n- 2026-11-01 | lane=a | rows=1 | pct=10%\n- 2026-11-02 | lane=a | rows=2 | pct=20%\n",
        )
        .unwrap();
        std::fs::write(dir.join("notes.md"), "- 2026-11-02 | lane=skip | rows=9\n").unwrap();
        let rows = read_lane_dir(&dir);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].lane, "a");
        assert_eq!(rows[0].rows, Some(2));
        assert_eq!(rows[0].pct, Some(20.0));
        assert_eq!(read_lane_dir(&dir.join("missing")).len(), 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn git_line_parses_unit_separator_fields() {
        let c = parse_git_line("abc123\x1fFix the wheel\x1f2026-11-02T08:00:00+00:00").unwrap();
        assert_eq!(c.sha, "abc123");
        assert_eq!(c.msg, "Fix the wheel");
        assert!(parse_git_line("").is_none());
        assert!(parse_git_line("\x1fno sha\x1fnow").is_none());
    }

    #[test]
    fn junit_attrs_read_counts_leniently() {
        let t = parse_junit_attrs(r#"<testsuite name="x" tests="30" failures="1" errors="0" timestamp="2026-11-02T08:00:00">"#).unwrap();
        assert_eq!((t.tests, t.failures, t.errors), (30, 1, 0));
        assert_eq!(t.at, "2026-11-02T08:00:00");
        assert!(parse_junit_attrs("<nope/>").is_none());
        let bare = parse_junit_attrs("<testsuite>").unwrap();
        assert_eq!((bare.tests, bare.failures, bare.errors), (0, 0, 0));
    }
}

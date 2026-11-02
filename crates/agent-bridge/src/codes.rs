//! Codes. It issues one-time confirm codes.
//! Each code lives 60 seconds and works once.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Code check failures. All map to 403 with plain words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeErr {
    /// No such code exists, or the pane or text differs.
    Missing,
    /// The code is older than its TTL.
    Expired,
    /// The code already sent once.
    Used,
}

/// One issued code. It binds exact text to one pane.
struct Entry {
    /// Pane the code was issued for.
    pane: String,
    /// Exact text that will send. Byte match is required.
    text: String,
    /// Time the code dies.
    expires: Instant,
    /// True after the first send attempt.
    used: bool,
}

/// One-time code store. It prunes dead codes on each call.
pub struct CodeStore {
    /// Codes by code string.
    codes: Mutex<HashMap<String, Entry>>,
    /// Code life in seconds.
    ttl_secs: u64,
}

impl CodeStore {
    /// Build a store with the given TTL.
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            codes: Mutex::new(HashMap::new()),
            ttl_secs,
        }
    }

    /// Issue a code for exact text to one pane. It returns code plus TTL.
    pub fn issue(&self, pane: &str, text: &str) -> (String, u64) {
        let code = uuid::Uuid::new_v4().simple().to_string();
        let mut codes = self.codes.lock().unwrap();
        prune(&mut codes);
        codes.insert(
            code.clone(),
            Entry {
                pane: pane.to_string(),
                text: text.to_string(),
                expires: Instant::now() + Duration::from_secs(self.ttl_secs),
                used: false,
            },
        );
        (code, self.ttl_secs)
    }

    /// Check a code without consuming it. Refusals keep the code live.
    pub fn check(&self, pane: &str, code: &str, text: &str) -> Result<(), CodeErr> {
        let mut codes = self.codes.lock().unwrap();
        let now = Instant::now();
        let Some(entry) = codes.get(code) else {
            prune(&mut codes);
            return Err(CodeErr::Missing);
        };
        if entry.expires <= now {
            codes.remove(code);
            return Err(CodeErr::Expired);
        }
        if entry.used {
            return Err(CodeErr::Used);
        }
        if entry.pane != pane || entry.text != text {
            return Err(CodeErr::Missing);
        }
        prune(&mut codes);
        Ok(())
    }

    /// Consume a code for one send attempt. Later uses fail.
    pub fn consume(&self, code: &str) {
        let mut codes = self.codes.lock().unwrap();
        if let Some(entry) = codes.get_mut(code) {
            entry.used = true;
        }
    }
}

/// Drop used and expired codes. Callers hold the lock.
fn prune(codes: &mut HashMap<String, Entry>) {
    let now = Instant::now();
    codes.retain(|_, e| !e.used && e.expires > now);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_code_checks_and_consumes_once() {
        let store = CodeStore::new(60);
        let (code, ttl) = store.issue("w1:p1", "hi");
        assert_eq!(ttl, 60);
        assert_eq!(store.check("w1:p1", &code, "hi"), Ok(()));
        store.consume(&code);
        assert_eq!(store.check("w1:p1", &code, "hi"), Err(CodeErr::Used));
    }

    #[test]
    fn wrong_pane_or_text_misses() {
        let store = CodeStore::new(60);
        let (code, _) = store.issue("w1:p1", "hi");
        assert_eq!(store.check("w1:p2", &code, "hi"), Err(CodeErr::Missing));
        assert_eq!(store.check("w1:p1", &code, "bye"), Err(CodeErr::Missing));
        assert_eq!(
            store.check("w1:p1", "no-such-code", "hi"),
            Err(CodeErr::Missing)
        );
    }

    #[test]
    fn old_code_expires() {
        let store = CodeStore::new(1);
        let (code, _) = store.issue("w1:p1", "hi");
        std::thread::sleep(Duration::from_millis(1100));
        assert_eq!(store.check("w1:p1", &code, "hi"), Err(CodeErr::Expired));
    }
}

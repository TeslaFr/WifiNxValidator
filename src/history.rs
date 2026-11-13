use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::SystemTime,
};
#[derive(Clone, Debug)]
pub struct Entry {
    pub time: SystemTime,
    pub client: String,
    pub protocol: String,
    pub target: String,
    pub kind: String,
    pub result: String,
}
impl Entry {
    pub fn new(client: &str, protocol: &str, target: &str, kind: &str, result: &str) -> Self {
        Self {
            time: SystemTime::now(),
            client: client.into(),
            protocol: protocol.into(),
            target: target.into(),
            kind: kind.into(),
            result: result.into(),
        }
    }
}
#[derive(Clone, Default)]
pub struct History(Arc<Mutex<VecDeque<Entry>>>);
impl History {
    pub fn push(&self, entry: Entry) {
        let mut entries = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if entries.len() == 5000 {
            entries.pop_front();
        }
        entries.push_back(entry);
    }
    pub fn snapshot(&self) -> Vec<Entry> {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .cloned()
            .collect()
    }
    pub fn clear(&self) {
        self.0.lock().unwrap_or_else(|p| p.into_inner()).clear();
    }
}

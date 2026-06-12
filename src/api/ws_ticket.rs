// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Short-lived, single-use tickets for WebSocket console upgrades.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;

const DEFAULT_TTL: Duration = Duration::from_secs(60);

static TICKETS: Lazy<Mutex<HashMap<String, Instant>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn purge_expired(store: &mut HashMap<String, Instant>) {
    let now = Instant::now();
    store.retain(|_, exp| *exp > now);
}

/// Issue a one-time WebSocket ticket valid for `ttl` (defaults to 60s).
pub fn issue_ws_ticket(ttl: Option<Duration>) -> String {
    let ttl = ttl.unwrap_or(DEFAULT_TTL);
    let ticket = format!("{:032x}{:032x}", rand::random::<u128>(), rand::random::<u128>());
    let mut store = TICKETS.lock().expect("ws ticket lock");
    purge_expired(&mut store);
    store.insert(ticket.clone(), Instant::now() + ttl);
    ticket
}

/// Consume a ticket if valid. Returns true when the ticket was present and not expired.
pub fn consume_ws_ticket(ticket: &str) -> bool {
    if ticket.is_empty() {
        return false;
    }
    let mut store = TICKETS.lock().expect("ws ticket lock");
    purge_expired(&mut store);
    match store.remove(ticket) {
        Some(exp) => Instant::now() <= exp,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_is_single_use() {
        let ticket = issue_ws_ticket(Some(Duration::from_secs(30)));
        assert!(consume_ws_ticket(&ticket));
        assert!(!consume_ws_ticket(&ticket));
    }

    #[test]
    fn empty_ticket_rejected() {
        assert!(!consume_ws_ticket(""));
    }
}

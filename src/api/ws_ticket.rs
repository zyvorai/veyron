// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Short-lived, single-use tickets for WebSocket console upgrades.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::api::auth_context::AuthContext;
use once_cell::sync::Lazy;

const DEFAULT_TTL: Duration = Duration::from_secs(60);

static TICKETS: Lazy<Mutex<HashMap<String, (Instant, AuthContext)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn purge_expired(store: &mut HashMap<String, (Instant, AuthContext)>) {
    let now = Instant::now();
    store.retain(|_, (exp, _)| *exp > now);
}

/// Issue a one-time WebSocket ticket valid for `ttl` (defaults to 60s).
pub fn issue_ws_ticket(caller: AuthContext, ttl: Option<Duration>) -> String {
    let ttl = ttl.unwrap_or(DEFAULT_TTL);
    let ticket = format!(
        "{:032x}{:032x}",
        rand::random::<u128>(),
        rand::random::<u128>()
    );
    let mut store = TICKETS.lock().expect("ws ticket lock");
    purge_expired(&mut store);
    store.insert(ticket.clone(), (Instant::now() + ttl, caller));
    ticket
}

/// Consume a ticket if valid. Returns the original caller when valid.
pub fn consume_ws_ticket(ticket: &str) -> Option<AuthContext> {
    if ticket.is_empty() {
        return None;
    }
    let mut store = TICKETS.lock().expect("ws ticket lock");
    purge_expired(&mut store);
    match store.remove(ticket) {
        Some((exp, caller)) if Instant::now() <= exp => Some(caller),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_is_single_use() {
        let caller = AuthContext {
            role: crate::api::http_server::web::ApiRole::ReadOnly,
            subject: "reader".into(),
        };
        let ticket = issue_ws_ticket(caller.clone(), Some(Duration::from_secs(30)));
        assert_eq!(consume_ws_ticket(&ticket), Some(caller));
        assert!(consume_ws_ticket(&ticket).is_none());
    }

    #[test]
    fn expired_ticket_rejected() {
        let ctx = AuthContext {
            role: crate::api::http_server::web::ApiRole::Write,
            subject: "writer".into(),
        };
        let ticket = issue_ws_ticket(ctx, Some(Duration::ZERO));
        assert!(consume_ws_ticket(&ticket).is_none());
    }

    #[test]
    fn empty_ticket_rejected() {
        assert!(consume_ws_ticket("").is_none());
    }
}

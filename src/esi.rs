//! Background ESI lookups (server status, character names).

use crate::eve::Esi;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

pub enum Reply {
    Status(Esi, bool),
    Names(HashMap<String, String>),
}

pub struct Client {
    tx: Sender<Reply>,
    pub rx: Receiver<Reply>,
    ctx: egui::Context,
}

#[derive(Deserialize)]
struct NameEntry {
    id: u64,
    name: String,
    category: String,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .user_agent("eve-settings-manager (linux)")
        .build()
        .into()
}

impl Client {
    pub fn new(ctx: egui::Context) -> Self {
        let (tx, rx) = channel();
        Client { tx, rx, ctx }
    }

    pub fn status(&self, esi: Esi) {
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let ok = esi
                .status_url()
                .is_some_and(|u| agent().get(u).call().is_ok_and(|r| r.status() == 200));
            let _ = tx.send(Reply::Status(esi, ok));
            ctx.request_repaint();
        });
    }

    pub fn names(&self, esi: Esi, ids: Vec<String>) {
        let Some(url) = esi.names_url() else { return };
        if ids.is_empty() {
            return;
        }
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let nums: Vec<u64> = ids.iter().filter_map(|i| i.parse().ok()).collect();
            let res: Result<Vec<NameEntry>, ureq::Error> = agent()
                .post(url)
                .send_json(&nums)
                .and_then(|mut r| r.body_mut().read_json());
            if let Ok(list) = res {
                let map = list
                    .into_iter()
                    .filter(|e| e.category == "character")
                    .map(|e| (e.id.to_string(), e.name))
                    .collect();
                let _ = tx.send(Reply::Names(map));
                ctx.request_repaint();
            }
        });
    }
}

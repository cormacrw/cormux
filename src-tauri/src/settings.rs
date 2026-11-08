//! The user's settings the core acts on, read from the settings table. The webview writes them
//! under these keys.

use crate::store::Store;

const RUN_EVERYTHING: &str = "runEverything";
const EXPERIMENTAL_REFERENCES: &str = "experimentalReferences";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Settings {
    /// Approve every tool request without asking. Questions still wait for the user.
    pub run_everything: bool,
    /// The experimental References panel, and the tool that pins links to it.
    pub references: bool,
}

impl Settings {
    /// What's stored, with anything unreadable taken as off.
    pub fn load(store: &Store) -> Self {
        let on = |key: &str| match store.get_setting(key) {
            Ok(value) => value.as_deref() == Some("true"),
            Err(error) => {
                log::warn!("reading setting {key}: {error}");
                false
            }
        };
        Self {
            run_everything: on(RUN_EVERYTHING),
            references: on(EXPERIMENTAL_REFERENCES),
        }
    }

    #[cfg(test)]
    pub fn save(self, store: &Store) {
        let flag = |on: bool| if on { "true" } else { "false" };
        store
            .set_setting(RUN_EVERYTHING, flag(self.run_everything))
            .unwrap();
        store
            .set_setting(EXPERIMENTAL_REFERENCES, flag(self.references))
            .unwrap();
    }
}

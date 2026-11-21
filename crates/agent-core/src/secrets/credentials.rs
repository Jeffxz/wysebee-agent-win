//! The Windows Credential Manager: one generic credential, service [`CREDENTIAL_SERVICE`],
//! account [`super::credential_account`] (stored as the target `<account>.<service>`, the
//! `keyring` crate's form), in the signed-in person's own vault. **Not exercised by the
//! tests** (they must not touch the real one), so this file is the only one outside the
//! 100% line rule (`scripts/check.ps1`). Keep it this small.

use super::{SecretStore, CREDENTIAL_SERVICE};
use keyring::{Entry, Error};

pub struct CredentialManager {
    pub account: String,
}

impl CredentialManager {
    fn entry(&self) -> Result<Entry, String> {
        Entry::new(CREDENTIAL_SERVICE, &self.account).map_err(|e| format!("The Credential Manager could not be used: {e}"))
    }
}

impl SecretStore for CredentialManager {
    fn get(&self) -> Result<Option<String>, String> {
        match self.entry()?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(Error::NoEntry) => Ok(None),
            Err(Error::BadEncoding(_)) => Err("The Credential Manager item could not be read.".into()),
            Err(e) => Err(format!("The Credential Manager could not be read: {e}")),
        }
    }

    fn set(&self, secret: &str) -> Result<(), String> {
        self.entry()?.set_password(secret).map_err(|e| format!("The Credential Manager could not be written: {e}"))
    }

    fn delete(&self) -> Result<(), String> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("The Credential Manager could not be changed: {e}")),
        }
    }
}

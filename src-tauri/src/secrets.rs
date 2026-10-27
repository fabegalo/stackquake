//! Segredos ficam somente no chaveiro do sistema operacional, nunca em
//! arquivos, logs ou na interface.

use zeroize::Zeroizing;

use crate::error::{AppError, AppResult};

const SERVICE: &str = "dev.stackquake.desktop";

pub const REFRESH_TOKEN: &str = "google-refresh-token";
pub const ACCOUNT: &str = "google-account";

/// Chave do banco local de uma conta (uma chave por conta).
pub fn db_key_name(account_local_id: &str) -> String {
    format!("local-db-key-{account_local_id}")
}

fn entry(key: &str) -> AppResult<keyring::Entry> {
    Ok(keyring::Entry::new(SERVICE, key)?)
}

pub fn get(key: &str) -> AppResult<Option<Zeroizing<String>>> {
    match entry(key)?.get_password() {
        Ok(v) => Ok(Some(Zeroizing::new(v))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::from(e)),
    }
}

pub fn set(key: &str, value: &str) -> AppResult<()> {
    entry(key)?.set_password(value)?;
    Ok(())
}

pub fn delete(key: &str) -> AppResult<()> {
    match entry(key)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::from(e)),
    }
}

/// Bytes aleatórios do gerador do sistema operacional.
pub fn random_bytes<const N: usize>() -> AppResult<Zeroizing<[u8; N]>> {
    let mut buf = Zeroizing::new([0u8; N]);
    getrandom::fill(buf.as_mut())
        .map_err(|e| AppError::Other(format!("gerador aleatório: {e}")))?;
    Ok(buf)
}

/// Comparação em tempo constante (evita vazar o valor esperado por tempo de resposta).
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ct_eq_works() {
        assert!(ct_eq(b"abc", b"abc"));
        assert!(!ct_eq(b"abc", b"abd"));
        assert!(!ct_eq(b"abc", b"ab"));
    }
}

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use blake2::{digest::{consts::U32, Mac}, Blake2bMac};
use chrono::Local;
use rand::{thread_rng, RngCore};
use crate::models::{Result, TokenPayload, ERR_INVALID_ARGUMENT};

pub struct Token {
    pub(crate) signature: Vec<u8>,
    pub(crate) payload: Vec<u8>
}

impl ToString for Token {
    fn to_string(&self) -> String {
        let signature_string = URL_SAFE_NO_PAD.encode(&self.signature);
        let payload_string = URL_SAFE_NO_PAD.encode(&self.payload);
        format!("{}.{}", payload_string, signature_string)
    }
}

impl Token {
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<_> = s.split(".").collect();
        if parts.len() != 2 {
            return Err(ERR_INVALID_ARGUMENT.clone().with_message("Invalid token string.".into()));
        }
        let signature = URL_SAFE_NO_PAD.decode(parts[1]).map_err(|_| {
            ERR_INVALID_ARGUMENT.clone().with_message("Invalid token string.".into())
        })?;
        let payload = URL_SAFE_NO_PAD.decode(parts[0]).map_err(|_| {
            ERR_INVALID_ARGUMENT.clone().with_message("Invalid token string.".into())
        })?;
        Ok(Self {
            signature,
            payload
        })
    }
}

#[derive(Clone)]
pub struct TokenFactory {
    key: Vec<u8>
}

impl TokenFactory {
    pub fn new() -> Self {
        let mut key = vec![0u8, 16];
        thread_rng().fill_bytes(&mut key);
        Self {
            key
        }
    }
    pub fn create(&self, payload: &TokenPayload) -> Result<Token> {
        let payload = serde_json::to_vec(payload)?;
        let mut mac = Blake2bMac::<U32>::new_with_salt_and_personal(
            &self.key,
            &[],
            &[]
        ).unwrap();
        mac.update(&payload);
        let signature = mac.finalize().into_bytes().to_vec();
        Ok(Token {
            signature,
            payload,
        })
    }
    pub fn parse(&self, token: &Token) -> Result<TokenPayload> {
        let mut mac = Blake2bMac::<U32>::new_with_salt_and_personal(
            &self.key,
            &[],
            &[]
        ).unwrap();
        mac.update(&token.payload);
        mac.verify_slice(&token.signature).map_err(|_|
            ERR_INVALID_ARGUMENT.clone().with_message("Invalid token signature".into())
        )?;
        let payload: TokenPayload = serde_json::from_slice(&token.payload)?;
        if payload.expires < Local::now() {
            return Err(ERR_INVALID_ARGUMENT.clone().with_message("Token has expired.".into()));
        }
        return Ok(payload)
    }
}
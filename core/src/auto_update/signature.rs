//! Ed25519 signature verification for update packages

use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use hex;
use std::fmt;

pub struct SignatureVerifier {
    public_key: VerifyingKey,
}

impl SignatureVerifier {
    /// Create new verifier with hex-encoded public key
    pub fn new(hex_public_key: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(hex_public_key.trim())
            .map_err(|e| anyhow::anyhow!("Invalid public key hex: {}", e))?;

        let key_bytes: [u8; 32] = bytes.try_into()
            .map_err(|_| anyhow::anyhow!("Invalid public key length"))?;
        let public_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|_| anyhow::anyhow!("Invalid Ed25519 public key"))?;

        Ok(Self { public_key })
    }

    /// Verify signature
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> anyhow::Result<bool> {
        let sig = Signature::from_bytes(signature.try_into()
            .map_err(|_| anyhow::anyhow!("Invalid signature length"))?);

        match self.public_key.verify(data, &sig) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Verify signature from hex string
    pub fn verify_hex(&self, data: &[u8], signature_hex: &str) -> anyhow::Result<bool> {
        let signature = hex::decode(signature_hex.trim())
            .map_err(|e| anyhow::anyhow!("Invalid signature hex: {}", e))?;

        self.verify(data, &signature)
    }

    /// Verify detached signature file
    pub fn verify_detached(&self, data: &[u8], signature_file_content: &[u8]) -> anyhow::Result<bool> {
        // Try different formats

        // 1. Raw binary signature
        if signature_file_content.len() == 64 {
            return self.verify(data, signature_file_content);
        }

        // 2. Hex-encoded signature
        if let Ok(decoded) = hex::decode(signature_file_content) {
            if decoded.len() == 64 {
                return self.verify(data, &decoded);
            }
        }

        // 3. Base64-encoded signature
        if let Ok(decoded) = base64::decode(signature_file_content) {
            if decoded.len() == 64 {
                return self.verify(data, &decoded);
            }
        }

        // 4. JSON format: {"signature": "..."}
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(signature_file_content) {
            if let Some(sig) = json.get("signature").and_then(|s| s.as_str()) {
                return self.verify_hex(data, sig);
            }
        }

        Err(anyhow::anyhow!("Unknown signature format"))
    }

    /// Get public key fingerprint
    pub fn fingerprint(&self) -> String {
        let bytes = self.public_key.to_bytes();
        let hex = hex::encode(&bytes);
        format!("ed25519:{:.16}...{:.16}", &hex[..16], &hex[hex.len()-16..])
    }
}

impl fmt::Debug for SignatureVerifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignatureVerifier")
            .field("fingerprint", &self.fingerprint())
            .finish()
    }
}

/// Key rotation support
pub struct KeyManager {
    primary_key: VerifyingKey,
    backup_keys: Vec<VerifyingKey>,
    valid_until: Option<std::time::SystemTime>,
}

impl KeyManager {
    pub fn new(primary_hex: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(primary_hex.trim())?;
        let key_bytes: [u8; 32] = bytes.try_into()
            .map_err(|_| anyhow::anyhow!("Invalid primary key length"))?;
        let primary_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|_| anyhow::anyhow!("Invalid primary key"))?;

        Ok(Self {
            primary_key,
            backup_keys: Vec::new(),
            valid_until: None,
        })
    }

    pub fn add_backup_key(&mut self, hex_key: &str) -> anyhow::Result<()> {
        let bytes = hex::decode(hex_key.trim())?;
        let key_bytes: [u8; 32] = bytes.try_into()
            .map_err(|_| anyhow::anyhow!("Invalid backup key length"))?;
        let key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|_| anyhow::anyhow!("Invalid backup key"))?;
        self.backup_keys.push(key);
        Ok(())
    }

    /// Verify with any valid key
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> bool {
        let sig_bytes: &[u8; 64] = match signature.try_into() {
            Ok(s) => s,
            Err(_) => return false,
        };
        let sig = Signature::from_bytes(sig_bytes);

        // Try primary key
        if self.primary_key.verify(data, &sig).is_ok() {
            return true;
        }

        // Try backup keys
        for key in &self.backup_keys {
            if key.verify(data, &sig).is_ok() {
                return true;
            }
        }

        false
    }
}

/// Certificate chain verification for enterprise deployments
#[derive(Debug, Clone)]
pub struct CertificateChain {
    pub certificates: Vec<Certificate>,
}

#[derive(Debug, Clone)]
pub struct Certificate {
    pub subject: String,
    pub issuer: String,
    pub public_key: Vec<u8>,
    pub not_before: u64,
    pub not_after: u64,
    pub signature: Vec<u8>,
}

impl CertificateChain {
    /// Verify chain against trusted root
    pub fn verify(&self, root_key: &VerifyingKey) -> anyhow::Result<bool> {
        if self.certificates.is_empty() {
            return Ok(false);
        }

        // Verify chain from leaf to root
        for i in 0..self.certificates.len() {
            let cert = &self.certificates[i];

            // Check expiration
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();

            if now < cert.not_before || now > cert.not_after {
                return Ok(false);
            }

            // Verify signature
            let signing_key = if i == self.certificates.len() - 1 {
                // Root certificate
                root_key
            } else {
                // Issuer's public key
                let issuer_cert = &self.certificates[i + 1];
                let key_bytes: [u8; 32] = issuer_cert.public_key.as_slice().try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid issuer public key length"))?;
                &VerifyingKey::from_bytes(&key_bytes)?
            };

            if !self.verify_cert_signature(cert, signing_key) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn verify_cert_signature(&self, cert: &Certificate, key: &VerifyingKey) -> bool {
        // In real implementation, this would verify the certificate signature
        // For now, simplified check
        let data = format!("{}|{}|{}", cert.subject, cert.issuer, hex::encode(&cert.public_key));

        if let Ok(sig_bytes) = <[u8; 64]>::try_from(&cert.signature[..]) {
            let sig = Signature::from_bytes(&sig_bytes);
            key.verify(data.as_bytes(), &sig).is_ok()
        } else {
            false
        }
    }
}

/// Secure enclave / TPM integration stub
#[cfg(feature = "secure-enclave")]
pub mod secure_enclave {
    use super::*;

    /// Verify signature using hardware security module
    pub fn hsm_verify(data: &[u8], signature: &[u8]) -> anyhow::Result<bool> {
        // Integration with TPM, Apple Secure Enclave, etc.
        // This is a stub for actual hardware integration
        log::info!("HSM verification requested (stub)");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    #[test]
    fn test_signature_verification() {
        // Generate test keypair
        let mut csprng = OsRng {};
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        let public_key_hex = hex::encode(verifying_key.to_bytes());

        // Create verifier
        let verifier = SignatureVerifier::new(&public_key_hex).unwrap();

        // Sign test data
        let data = b"test update package";
        let signature = signing_key.sign(data);

        // Verify
        assert!(verifier.verify(data, &signature.to_bytes()).unwrap());

        // Verify with wrong data should fail
        assert!(!verifier.verify(b"wrong data", &signature.to_bytes()).unwrap());
    }

    #[test]
    fn test_hex_signature() {
        let mut csprng = OsRng {};
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        let public_key_hex = hex::encode(verifying_key.to_bytes());
        let verifier = SignatureVerifier::new(&public_key_hex).unwrap();

        let data = b"test data";
        let signature = signing_key.sign(data);
        let sig_hex = hex::encode(signature.to_bytes());

        assert!(verifier.verify_hex(data, &sig_hex).unwrap());
    }
}

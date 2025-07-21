use rustls::{
    DigitallySignedStruct, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
};
use rustls_pki_types::{CertificateDer, ServerName, UnixTime};
use std::{
    collections::HashSet,
    fs,
    io::{self, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug)]
pub struct PromptingCertVerifier {
    trusted_self_signed: Arc<Mutex<HashSet<Vec<u8>>>>,
    trusted_certs_dir: PathBuf,
}

impl PromptingCertVerifier {
    pub fn new() -> Self {
        Self::with_trust_dir(None)
    }

    pub fn with_trust_dir(trust_dir: Option<PathBuf>) -> Self {
        let trusted_certs_dir = trust_dir.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".bigbot")
                .join("trusted_certificates")
        });

        // Ensure the trusted certificates directory exists
        if let Err(e) = fs::create_dir_all(&trusted_certs_dir) {
            eprintln!("Warning: Failed to create trusted certificates directory: {}", e);
        }

        let mut verifier = Self {
            trusted_self_signed: Arc::new(Mutex::new(HashSet::new())),
            trusted_certs_dir,
        };

        // Load existing trusted certificates
        verifier.load_trusted_certificates();
        verifier
    }

    fn load_trusted_certificates(&mut self) {
        if let Ok(entries) = fs::read_dir(&self.trusted_certs_dir) {
            let mut trusted = self.trusted_self_signed.lock().unwrap();
            for entry in entries.flatten() {
                if let Some(extension) = entry.path().extension() {
                    if extension == "der" {
                        if let Ok(cert_data) = fs::read(entry.path()) {
                            trusted.insert(cert_data);
                            if let Some(filename) = entry.path().file_name() {
                                println!("Loaded trusted certificate: {:?}", filename);
                            }
                        }
                    }
                }
            }
        }
    }

    fn save_certificate(&self, cert: &CertificateDer, fingerprint: &str) -> Result<(), Box<dyn std::error::Error>> {
        let filename = format!("{}.der", fingerprint.replace(":", ""));
        let cert_path = self.trusted_certs_dir.join(filename);
        
        fs::write(&cert_path, cert.as_ref())?;
        println!("Saved trusted certificate to: {:?}", cert_path);
        Ok(())
    }

    fn prompt_user(&self, fingerprint: &str) -> bool {
        println!(
            "Encountered self-signed certificate with fingerprint:\n  {}",
            fingerprint
        );
        print!("Do you want to trust this certificate? [y/N]: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
    }

    fn fingerprint(cert: &CertificateDer) -> String {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(&cert.to_vec());
        hash.iter()
            .map(|b| format!("{:02X}:", b))
            .collect::<String>()
            .trim_end_matches(':')
            .to_string()
    }
}

impl ServerCertVerifier for PromptingCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer,
        _intermediates: &[CertificateDer],
        _server_name: &ServerName,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let cert = _end_entity;
        let mut trusted = self.trusted_self_signed.lock().unwrap();

        if trusted.contains(&cert.to_vec()) {
            return Ok(ServerCertVerified::assertion());
        }

        let fp = Self::fingerprint(cert);
        if self.prompt_user(&fp) {
            trusted.insert(cert.to_vec());
            // Save the certificate to disk for future use
            if let Err(e) = self.save_certificate(cert, &fp) {
                eprintln!("Warning: Failed to save trusted certificate: {}", e);
            }
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "User rejected self-signed certificate".into(),
            ))
        }
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ED25519,
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::RSA_PSS_SHA256,
        ]
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _signed: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        // Simplified: Accept all for now (but you can validate with ring or webpki if desired)
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _signed: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        // Simplified: Accept all for now (but you can validate with ring or webpki if desired)
        Ok(HandshakeSignatureValid::assertion())
    }
}

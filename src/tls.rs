use axum_server::tls_rustls::RustlsConfig;
use rustls_acme::{
    caches::DirCache,
    rustls::{
        self,
        pki_types::{pem::PemObject, CertificateDer},
    },
    AcmeConfig, AcmeState,
};
use std::{io, path::PathBuf, sync::Arc};

/// TLS is opt-in; ACME and a provided certificate are mutually exclusive.
#[derive(clap::Args, Default)]
pub struct TlsOptions {
    /// PEM certificate chain for HTTPS (requires --tls-key).
    #[arg(long, value_name = "PEM")]
    pub tls_cert: Option<PathBuf>,
    /// PEM private key for HTTPS (requires --tls-cert).
    #[arg(long, value_name = "PEM")]
    pub tls_key: Option<PathBuf>,
    /// DNS name to obtain a certificate for; repeat for additional names.
    #[arg(long, value_name = "DOMAIN")]
    pub acme_domain: Vec<String>,
    /// ACME contact email; repeat for additional contacts.
    #[arg(long, value_name = "EMAIL")]
    pub acme_contact: Vec<String>,
    /// Persistent private directory for ACME certificates and account keys.
    #[arg(long, value_name = "DIRECTORY")]
    pub acme_cache: Option<PathBuf>,
    /// Use Let's Encrypt staging (certificates are not publicly trusted).
    #[arg(long)]
    pub acme_staging: bool,
    /// Custom HTTPS ACME directory (cannot be combined with --acme-staging).
    #[arg(long, value_name = "HTTPS_URL")]
    pub acme_directory: Option<String>,
    /// PEM roots trusted only for the custom ACME directory's HTTPS API.
    #[arg(long, value_name = "PEM")]
    pub acme_ca_cert: Option<PathBuf>,
}

pub enum TlsConfig {
    Provided(RustlsConfig),
    Acme(AcmeState<io::Error, io::Error>),
}

impl TlsOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self.tls_cert.is_some() != self.tls_key.is_some() {
            return Err("--tls-cert and --tls-key must be supplied together".into());
        }
        let acme_options = !self.acme_domain.is_empty()
            || !self.acme_contact.is_empty()
            || self.acme_cache.is_some()
            || self.acme_staging
            || self.acme_directory.is_some()
            || self.acme_ca_cert.is_some();
        if self.tls_cert.is_some() && acme_options {
            return Err("provided PEM certificates cannot be combined with ACME options".into());
        }
        if acme_options {
            if self.acme_staging && self.acme_directory.is_some() {
                return Err("--acme-directory cannot be combined with --acme-staging".into());
            }
            if self.acme_ca_cert.is_some() && self.acme_directory.is_none() {
                return Err("--acme-ca-cert requires --acme-directory".into());
            }
            if let Some(directory) = &self.acme_directory {
                let uri = directory
                    .parse::<hyper::Uri>()
                    .map_err(|_| "--acme-directory must be an absolute HTTPS URL")?;
                if uri.scheme_str() != Some("https")
                    || uri.host().is_none_or(str::is_empty)
                    || uri
                        .authority()
                        .is_some_and(|authority| authority.as_str().contains('@'))
                {
                    return Err(
                        "--acme-directory must be an absolute HTTPS URL without credentials".into(),
                    );
                }
            }
            if self.acme_domain.is_empty()
                || self.acme_contact.is_empty()
                || self.acme_cache.is_none()
            {
                return Err("ACME requires --acme-domain, --acme-contact and --acme-cache".into());
            }
            for domain in &self.acme_domain {
                if !valid_dns_name(domain) {
                    return Err(format!("invalid ACME DNS name '{domain}'; use an ASCII public DNS name, not an IP address or wildcard"));
                }
            }
            for contact in &self.acme_contact {
                let email = contact.strip_prefix("mailto:").unwrap_or(contact);
                let valid = email
                    .split_once('@')
                    .is_some_and(|(local, domain)| !local.is_empty() && valid_dns_name(domain))
                    && !email.chars().any(char::is_whitespace)
                    && !email.contains(['?', '#', ',']);
                if !valid {
                    return Err(format!("invalid ACME contact email '{contact}'"));
                }
            }
            if self
                .acme_cache
                .as_ref()
                .is_some_and(|path| path.as_os_str().is_empty())
            {
                return Err("--acme-cache cannot be empty".into());
            }
        }
        Ok(())
    }

    /// Load keys and validate the cache before starting Zend or the listener.
    pub async fn prepare(self) -> Result<Option<TlsConfig>, Box<dyn std::error::Error>> {
        self.validate()?;
        if let (Some(cert), Some(key)) = (self.tls_cert, self.tls_key) {
            return Ok(Some(TlsConfig::Provided(
                RustlsConfig::from_pem_file(cert, key).await?,
            )));
        }
        if self.acme_domain.is_empty() {
            return Ok(None);
        }
        let cache = self.acme_cache.expect("validated ACME cache");
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&cache)?;
        let metadata = std::fs::metadata(&cache)?;
        if !metadata.is_dir() || metadata.permissions().readonly() {
            return Err("ACME cache must be a writable directory".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(format!(
                    "ACME cache '{}' contains private keys and must have mode 0700",
                    cache.display()
                )
                .into());
            }
        }
        let contacts = self.acme_contact.iter().map(|email| {
            if email.starts_with("mailto:") {
                email.clone()
            } else {
                format!("mailto:{email}")
            }
        });
        let mut config = AcmeConfig::new(self.acme_domain)
            .contact(contacts)
            .cache(DirCache::new(cache))
            .directory_lets_encrypt(!self.acme_staging);
        if let Some(directory) = self.acme_directory {
            config = config.directory(directory);
        }
        if let Some(path) = self.acme_ca_cert {
            let mut roots = rustls::RootCertStore::empty();
            for certificate in CertificateDer::pem_file_iter(path)? {
                roots.add(certificate?)?;
            }
            if roots.is_empty() {
                return Err("--acme-ca-cert must contain at least one CA certificate".into());
            }
            let client = rustls::ClientConfig::builder_with_provider(
                rustls::crypto::ring::default_provider().into(),
            )
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots)
            .with_no_client_auth();
            config = config.client_tls_config(Arc::new(client));
        }
        Ok(Some(TlsConfig::Acme(config.state())))
    }
}

fn valid_dns_name(domain: &str) -> bool {
    domain.len() <= 253
        && domain.contains('.')
        && domain.parse::<std::net::IpAddr>().is_err()
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acme_options() -> TlsOptions {
        TlsOptions {
            acme_domain: vec!["example.com".into()],
            acme_contact: vec!["admin@example.com".into()],
            acme_cache: Some("cache".into()),
            ..Default::default()
        }
    }

    #[test]
    fn rejects_incomplete_and_conflicting_tls_modes() {
        let options = TlsOptions {
            tls_cert: Some("cert.pem".into()),
            ..Default::default()
        };
        assert!(options.validate().is_err());
        let mut options = acme_options();
        options.acme_contact.clear();
        assert!(options.validate().is_err());
        let mut options = acme_options();
        options.tls_cert = Some("cert.pem".into());
        options.tls_key = Some("key.pem".into());
        assert!(options.validate().is_err());
    }

    #[test]
    fn validates_custom_directory_and_scoped_trust() {
        let mut options = acme_options();
        options.acme_ca_cert = Some("ca.pem".into());
        assert!(options.validate().is_err());
        options.acme_directory = Some("https://localhost:14000/dir".into());
        assert!(options.validate().is_ok());
        options.acme_staging = true;
        assert!(options.validate().is_err());
        options.acme_staging = false;
        for directory in [
            "http://localhost/dir",
            "/dir",
            "https://user@localhost/dir",
            "",
        ] {
            options.acme_directory = Some(directory.into());
            assert!(options.validate().is_err(), "{directory}");
        }
    }

    #[test]
    fn rejects_invalid_acme_domains() {
        for domain in [
            "localhost",
            "127.0.0.1",
            "*.example.com",
            "https://example.com",
            "-bad.example.com",
            "example.com.",
        ] {
            let mut options = acme_options();
            options.acme_domain = vec![domain.into()];
            assert!(options.validate().is_err(), "{domain}");
        }
    }
}

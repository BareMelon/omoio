//! The official updates Sony published for a game.
//!
//! This is the one place Omoio fetches game content, and it is the content the
//! user's own copy of the game is entitled to: the publisher's own patches for
//! a disc they own. Nothing here decrypts anything or works around any
//! protection.
//!
//! Two hosts, two different problems:
//!
//! - The manifest is served over HTTPS by a host whose certificate chains to
//!   Sony's own private root, which no public trust store carries, and that
//!   root signs with SHA-1 so a modern TLS stack rejects it twice over. Rather
//!   than turn certificate checking off, which is what comparable tools do,
//!   this pins the server's public key. Only Sony's key gets through, and if
//!   they rotate it the fetch fails loudly instead of quietly trusting anyone.
//! - The packages themselves are plain HTTP, so there is nothing to verify at
//!   the transport. Every one carries a SHA-1 in the manifest, and a download
//!   that does not match it is thrown away.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

const MANIFEST_HOST: &str = "a0.ww.np.dl.playstation.net";

/// A package ends with a footer that the published checksum does not cover.
///
/// Found by measuring rather than by reading it somewhere: the real 01.33
/// package for BCES00850 arrives at exactly the size the manifest states, and
/// the SHA-1 of the whole file does not match, while the SHA-1 of everything
/// but the last 32 bytes matches exactly.
const FOOTER: usize = 32;

/// SHA-256 of the server's public key, base64, as a browser would pin it.
///
/// Taken from the live certificate for `*.ww.np.dl.playstation.net`, which
/// Sony issued in May 2025 and which runs to July 2028. Pinning the key rather
/// than the certificate means a re-issue with the same key keeps working.
const PINNED_KEY: &str = "RKM381gVfu5xg8EQ2cvZ+dOSxlat56AxiT0Kq8nF7KA=";

fn manifest_url(title_id: &str) -> String {
    format!("https://{MANIFEST_HOST}/tpl/np/{title_id}/{title_id}-ver.xml")
}

/// One published update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Update {
    pub version: String,
    pub size: u64,
    pub sha1: String,
    pub url: String,
    /// The firmware this update expects. Shown so a failure to run afterwards
    /// is not a mystery.
    pub firmware: String,
}

/// Reads the packages out of Sony's manifest.
///
/// The shape was taken from a real response rather than assumed: a
/// `titlepatch` element holding `tag` elements, each holding `package`
/// elements with version, size, sha1sum and url attributes.
fn parse(xml: &str) -> Vec<Update> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut updates = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) if e.name().as_ref() == b"package" => {
                let mut update = Update {
                    version: String::new(),
                    size: 0,
                    sha1: String::new(),
                    url: String::new(),
                    firmware: String::new(),
                };
                for attribute in e.attributes().flatten() {
                    let value = attribute.unescape_value().unwrap_or_default().into_owned();
                    match attribute.key.as_ref() {
                        b"version" => update.version = value,
                        b"size" => update.size = value.parse().unwrap_or(0),
                        b"sha1sum" => update.sha1 = value,
                        b"url" => update.url = value,
                        b"ps3_system_ver" => update.firmware = value,
                        _ => {}
                    }
                }
                if !update.version.is_empty() && !update.url.is_empty() {
                    updates.push(update);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    // Newest first, so the one most people want is at the top.
    updates.reverse();
    updates
}

/// The build carries two crypto providers, so rustls refuses to pick one on
/// its own. Naming it here keeps that choice explicit and out of global state.
fn provider() -> std::sync::Arc<rustls::crypto::CryptoProvider> {
    std::sync::Arc::new(rustls::crypto::ring::default_provider())
}

/// Accepts exactly one public key and nothing else.
///
/// This deliberately does not walk the certificate chain. The chain ends at a
/// root signed with SHA-1 that no modern verifier will accept, so checking it
/// is not an option that exists. Matching the key is stronger than what the
/// chain would have told us anyway: it admits one specific server rather than
/// anything a trusted authority chose to vouch for.
#[derive(Debug)]
struct PinnedKey {
    provider: std::sync::Arc<rustls::crypto::CryptoProvider>,
}

impl PinnedKey {
    fn matches(cert: &rustls::pki_types::CertificateDer<'_>) -> bool {
        use sha2::{Digest, Sha256};
        let Ok((_, parsed)) = x509_public_key(cert) else {
            return false;
        };
        let digest = Sha256::digest(parsed);
        base64_encode(&digest) == PINNED_KEY
    }
}

/// Pulls the SubjectPublicKeyInfo out of a DER certificate.
///
/// A certificate is a SEQUENCE whose first element is the TBSCertificate, and
/// SubjectPublicKeyInfo is the last SEQUENCE inside it before the extensions.
/// Rather than carry an X.509 parser for one field, this walks the DER far
/// enough to find it, and refuses anything it does not recognise.
fn x509_public_key(cert: &[u8]) -> Result<((), &[u8]), ()> {
    // SubjectPublicKeyInfo always begins with a SEQUENCE holding the algorithm
    // identifier, and for these certificates that is rsaEncryption. Finding
    // that OID locates the field without decoding everything before it.
    const RSA_OID: &[u8] = &[
        0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01, 0x05, 0x00,
    ];
    let start = cert
        .windows(RSA_OID.len())
        .position(|window| window == RSA_OID)
        .ok_or(())?;

    // Step back to the SEQUENCE header that opens SubjectPublicKeyInfo.
    let (header, length) = der_header_before(cert, start)?;
    Ok(((), &cert[header..header + length]))
}

/// Finds the SEQUENCE header immediately preceding `inner`, and returns where
/// it starts and how long the whole element is.
fn der_header_before(der: &[u8], inner: usize) -> Result<(usize, usize), ()> {
    for start in (0..inner).rev() {
        if der[start] != 0x30 {
            continue;
        }
        let Some((length, header_len)) = der_length(&der[start + 1..]) else {
            continue;
        };
        let total = 1 + header_len + length;
        // The element has to begin just before the algorithm identifier and
        // cover it, which pins down the right SEQUENCE.
        if start + 1 + header_len == inner && start + total <= der.len() {
            return Ok((start, total));
        }
    }
    Err(())
}

fn der_length(bytes: &[u8]) -> Option<(usize, usize)> {
    let first = *bytes.first()?;
    if first < 0x80 {
        return Some((first as usize, 1));
    }
    let count = (first & 0x7f) as usize;
    if count == 0 || count > 4 || bytes.len() < count + 1 {
        return None;
    }
    let mut length = 0usize;
    for byte in &bytes[1..=count] {
        length = (length << 8) | *byte as usize;
    }
    Some((length, count + 1))
}

fn base64_encode(bytes: &[u8]) -> String {
    const SET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(SET[(n >> 18) as usize & 63] as char);
        out.push(SET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { SET[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { SET[n as usize & 63] as char } else { '=' });
    }
    out
}

impl rustls::client::danger::ServerCertVerifier for PinnedKey {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        if Self::matches(end_entity) {
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "the update server presented an unexpected key".into(),
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}

fn pinned_client() -> Result<reqwest::Client, String> {
    let provider = provider();
    let config = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|_| "Couldn't set up a secure connection.".to_string())?
        .dangerous()
        .with_custom_certificate_verifier(std::sync::Arc::new(PinnedKey { provider }))
        .with_no_client_auth();

    reqwest::Client::builder()
        .use_preconfigured_tls(config)
        .user_agent(super::USER_AGENT)
        .build()
        .map_err(|_| "Couldn't reach Sony's update service.".to_string())
}

/// Every update Sony has published for this game, newest first.
pub async fn available(title_id: &str) -> Result<Vec<Update>, String> {
    let response = pinned_client()?
        .get(manifest_url(title_id))
        .send()
        .await
        .map_err(|_| "Couldn't reach Sony's update service.".to_string())?;

    // A game with no updates answers with a document saying so, not an error.
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }
    if !response.status().is_success() {
        return Err("Sony's update service isn't answering right now.".into());
    }

    let xml = response
        .text()
        .await
        .map_err(|_| "Sony's update service sent something we couldn't read.".to_string())?;
    Ok(parse(&xml))
}

fn downloads_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(super::install_dir(app)?.join("omoio-updates"))
}

/// Downloads one update and hands it to RPCS3 to install.
///
/// The file is checked against the checksum Sony published before RPCS3 is
/// allowed near it, and thrown away afterwards either way.
pub async fn install(
    app: &AppHandle,
    update: &Update,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<(), String> {
    use futures_util::StreamExt;
    use sha1::{Digest, Sha1};
    use tokio::io::AsyncWriteExt;

    let dir = downloads_dir(app)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file_name = update
        .url
        .rsplit('/')
        .next()
        .filter(|name| name.ends_with(".pkg"))
        .ok_or("That update's address doesn't look right.")?;
    let path = dir.join(file_name);

    let response = reqwest::Client::new()
        .get(&update.url)
        .header("User-Agent", super::USER_AGENT)
        .send()
        .await
        .map_err(|_| "Couldn't download the update.".to_string())?;
    if !response.status().is_success() {
        return Err("Couldn't download the update.".into());
    }

    let total = response.content_length().unwrap_or(update.size);
    let mut file = tokio::fs::File::create(&path).await.map_err(|e| e.to_string())?;
    let mut hasher = Sha1::new();
    // The published checksum covers everything but the package's own trailing
    // footer, so the last FOOTER bytes are held back and never hashed. Checked
    // against a real package: hashing the whole file does not match, and
    // hashing all but the last 32 bytes does.
    let mut tail: Vec<u8> = Vec::with_capacity(FOOTER * 2);
    let mut done = 0u64;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            let _ = std::fs::remove_file(&path);
            return Err("cancelled".into());
        }
        let chunk = chunk.map_err(|_| "The download stopped early.".to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        done += chunk.len() as u64;

        // Only bytes that can no longer turn out to be the footer are hashed.
        tail.extend_from_slice(&chunk);
        if tail.len() > FOOTER {
            let upto = tail.len() - FOOTER;
            hasher.update(&tail[..upto]);
            tail.drain(..upto);
        }
        progress(done, total);
    }
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);

    let got: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if !got.eq_ignore_ascii_case(&update.sha1) {
        let _ = std::fs::remove_file(&path);
        return Err("The update didn't arrive intact. Try again.".into());
    }

    let outcome = super::launch::install_package(app, &path);
    let _ = std::fs::remove_file(&path);
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed from the real response for BCES00850.
    const MANIFEST: &str = r#"<?xml version='1.0' encoding='UTF-8'?>
<titlepatch status="alive" titleid="BCES00850"><tag name="BCES00850_T44" popup="true" signoff="true">
<package version="01.01" size="7865440" sha1sum="3e169188c0e46a490824901e627865c3f81d7fc3" url="http://b0.ww.np.dl.playstation.net/tppkg/np/BCES00850/x/a.pkg" ps3_system_ver="03.5000"/>
<package version="01.33" size="51273408" sha1sum="9ef3979e0220cc0193a680dca5dc5855f3ab5750" url="http://b0.ww.np.dl.playstation.net/tppkg/np/BCES00850/x/b.pkg" ps3_system_ver="04.2000"><paramsfo><TITLE>LittleBigPlanet 2</TITLE></paramsfo></package>
</tag></titlepatch>"#;

    #[test]
    fn reads_the_packages_out_of_a_real_manifest() {
        let updates = parse(MANIFEST);
        assert_eq!(updates.len(), 2);
        // Newest first.
        assert_eq!(updates[0].version, "01.33");
        assert_eq!(updates[0].size, 51273408);
        assert_eq!(updates[0].sha1, "9ef3979e0220cc0193a680dca5dc5855f3ab5750");
        assert!(updates[0].url.ends_with("b.pkg"));
        assert_eq!(updates[0].firmware, "04.2000");
        assert_eq!(updates[1].version, "01.01");
    }

    #[test]
    fn a_game_with_no_updates_is_not_an_error() {
        let empty = r#"<?xml version='1.0'?><titlepatch status="notfound" titleid="X"></titlepatch>"#;
        assert!(parse(empty).is_empty());
    }

    #[test]
    fn base64_matches_what_openssl_produces() {
        // The pin itself was produced by openssl; this checks our encoder
        // agrees with it rather than quietly producing something else.
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn der_lengths_are_read_the_way_der_writes_them() {
        assert_eq!(der_length(&[0x05]), Some((5, 1)));
        assert_eq!(der_length(&[0x81, 0x80]), Some((128, 2)));
        assert_eq!(der_length(&[0x82, 0x01, 0x22]), Some((290, 3)));
        // Indefinite and oversized lengths are not something we accept.
        assert_eq!(der_length(&[0x80]), None);
        assert_eq!(der_length(&[0x88, 1, 2, 3, 4, 5, 6, 7, 8]), None);
    }

    /// Reads the key out of the certificate Sony actually serves. Point
    /// OMOIO_SONY_CERT at a DER copy to run it.
    #[test]
    #[ignore]
    fn pins_the_key_sony_is_serving() {
        let path = std::env::var("OMOIO_SONY_CERT").expect("set OMOIO_SONY_CERT");
        let der = std::fs::read(path).expect("read the certificate");
        let cert = rustls::pki_types::CertificateDer::from(der);
        assert!(
            PinnedKey::matches(&cert),
            "the live key no longer matches the pin, so updates would stop"
        );
    }

    /// The checksum rule, run against a real package. Point OMOIO_PKG at a
    /// downloaded .pkg and OMOIO_PKG_SHA1 at the sha1sum the manifest gives
    /// for it.
    #[test]
    #[ignore]
    fn the_published_checksum_covers_everything_but_the_footer() {
        use sha1::{Digest, Sha1};

        let path = std::env::var("OMOIO_PKG").expect("set OMOIO_PKG");
        let expected = std::env::var("OMOIO_PKG_SHA1").expect("set OMOIO_PKG_SHA1");
        let bytes = std::fs::read(path).expect("read the package");

        let hex = |data: &[u8]| -> String {
            Sha1::digest(data).iter().map(|b| format!("{b:02x}")).collect()
        };

        assert_ne!(
            hex(&bytes),
            expected,
            "if the whole file matched, the footer rule would be wrong"
        );
        assert_eq!(
            hex(&bytes[..bytes.len() - FOOTER]),
            expected,
            "the checksum should cover everything but the last {FOOTER} bytes"
        );
    }

    /// Talks to Sony. This is the only check that the pin, the handshake and
    /// the parser all work against the real thing rather than a copy.
    #[test]
    #[ignore]
    fn reaches_sony_and_reads_what_they_published() {
        let updates = tauri::async_runtime::block_on(available("BCES00850"))
            .expect("the manifest should be reachable");

        assert!(!updates.is_empty(), "no updates came back");
        assert_eq!(updates[0].version, "01.33", "newest should be first");
        assert!(updates[0].url.starts_with("http://"), "packages are plain http");
        assert_eq!(updates[0].sha1.len(), 40, "sha1 should be 40 hex characters");
        // Versions must arrive newest first and never repeat.
        let mut seen: Vec<&String> = updates.iter().map(|u| &u.version).collect();
        let before = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(before, seen.len(), "the same version appeared twice");
    }
}

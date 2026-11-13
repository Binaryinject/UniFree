use std::fs;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::traits::PublicKeyParts;
use rsa::{Pkcs1v15Sign, RsaPrivateKey};
use sha1::{Digest, Sha1};

/// Sign the ULF content with an RSA key
fn sign_xml(content: &str, private_key: &RsaPrivateKey) -> Result<String, String> {
    let public_key = private_key.to_public_key();

    // Find the License element with id="Terms" for digest
    let terms_start = content.find("<License")
        .ok_or("No <License> element found")?;
    let terms_end = content.find("</License>")
        .ok_or("No </License> closing tag found")? + "</License>".len();
    let terms_xml = &content[terms_start..terms_end];

    // SHA1 digest of the License element (strip any existing Signature)
    let terms_clean = if let Some(sig_start) = terms_xml.find("<Signature") {
        let sig_end = terms_xml.find("</Signature>").ok_or("Malformed Signature")? + "</Signature>".len();
        format!("{}{}", &terms_xml[..sig_start], &terms_xml[sig_end..])
    } else {
        terms_xml.to_string()
    };

    let digest = Sha1::digest(terms_clean.as_bytes());
    let digest_b64 = BASE64.encode(digest);

    // Build SignedInfo
    let signed_info = format!(
        "<SignedInfo>\n  <CanonicalizationMethod Algorithm=\"http://www.w3.org/TR/2001/REC-xml-c14n-20010315#WithComments\"/>\n  <SignatureMethod Algorithm=\"http://www.w3.org/2000/09/xmldsig#rsa-sha1\"/>\n  <Reference URI=\"#Terms\">\n    <Transforms>\n      <Transform Algorithm=\"http://www.w3.org/2000/09/xmldsig#enveloped-signature\"/>\n    </Transforms>\n    <DigestMethod Algorithm=\"http://www.w3.org/2000/09/xmldsig#sha1\"/>\n    <DigestValue>{}</DigestValue>\n  </Reference>\n</SignedInfo>",
        digest_b64
    );

    // Sign the SignedInfo with RSA-SHA1
    let signed_info_hash = Sha1::digest(signed_info.as_bytes());
    let signature = private_key
        .sign(Pkcs1v15Sign::new::<Sha1>(), &signed_info_hash)
        .map_err(|e| format!("RSA sign failed: {}", e))?;
    let signature_b64 = BASE64.encode(signature);

    // Build complete Signature element
    let signature_xml = format!(
        "<Signature xmlns=\"http://www.w3.org/2000/09/xmldsig#\">\n{}\n  <SignatureValue>{}</SignatureValue>\n  <KeyInfo>\n    <KeyValue>\n      <RSAKeyValue>\n        <Modulus>{}</Modulus>\n        <Exponent>{}</Exponent>\n      </RSAKeyValue>\n    </KeyValue>\n  </KeyInfo>\n</Signature>",
        signed_info,
        signature_b64,
        BASE64.encode(public_key.n_bytes()),
        BASE64.encode(public_key.e_bytes())
    );

    let ulf_content = content.replace("</root>", &format!("{}</root>", signature_xml));
    Ok(ulf_content)
}

/// 将ALF文件转为ULF（用RSA密钥签名）
/// private_key_pem: Some(PEM string) 使用指定密钥, None 随机生成
pub fn sign_alf_to_ulf(
    alf_path: &Path,
    ulf_path: &Path,
    private_key_pem: Option<&str>,
) -> Result<String, String> {
    let alf_content = fs::read_to_string(alf_path)
        .map_err(|e| format!("Failed to read ALF: {}", e))?;

    // Get or generate RSA private key
    let private_key = match private_key_pem {
        Some(pem) if !pem.trim().is_empty() => {
            RsaPrivateKey::from_pkcs1_pem(pem)
                .map_err(|e| format!("Invalid RSA private key PEM: {}", e))?
        }
        _ => {
            eprintln!("Generating random RSA-2048 key...");
            RsaPrivateKey::new(&mut rand::rng(), 2048)
                .map_err(|e| format!("RSA keygen failed: {}", e))?
        }
    };

    let ulf_content = sign_xml(&alf_content, &private_key)?;

    // 确保父目录存在（macOS 的 /Library 需要提权，交给 write_ulf 处理）
    if let Some(parent) = ulf_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    write_ulf(ulf_path, &ulf_content)?;

    Ok(ulf_path.to_string_lossy().to_string())
}

/// 写入 ULF：普通路径直接写；macOS 的 `/Library/Application Support/Unity` 属 root，
/// 权限不足时通过 `osascript ... with administrator privileges` 弹窗授权后写入。
fn write_ulf(path: &Path, content: &str) -> Result<(), String> {
    match fs::write(path, content) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            #[cfg(target_os = "macos")]
            {
                write_ulf_with_admin(path, content)
            }
            #[cfg(not(target_os = "macos"))]
            {
                Err(format!("Failed to write ULF: {}", e))
            }
        }
        Err(e) => Err(format!("Failed to write ULF: {}", e)),
    }
}

#[cfg(target_os = "macos")]
fn write_ulf_with_admin(path: &Path, content: &str) -> Result<(), String> {
    let tmp = std::env::temp_dir().join("unifree_Unity_lic.ulf");
    fs::write(&tmp, content).map_err(|e| format!("Failed to stage ULF: {}", e))?;
    let dir = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("/Library/Application Support/Unity"));
    let quote = |p: &Path| p.to_string_lossy().replace('"', "\\\"");
    let script = format!(
        "do shell script \"mkdir -p \\\"{}\\\" && /bin/cp -f \\\"{}\\\" \\\"{}\\\" && /usr/sbin/chown root:wheel \\\"{}\\\" && /bin/chmod 644 \\\"{}\\\"\" with administrator privileges",
        quote(&dir),
        quote(&tmp),
        quote(path),
        quote(path),
        quote(path)
    );
    let out = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| format!("Failed to run osascript: {}", e))?;
    if !out.status.success() {
        return Err(format!(
            "Administrator authorization failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

use anyhow::{Context, Result};
use md5::{Digest, Md5};
use serde::Deserialize;

const DOWNLOAD_SECRET_SALT: &str = "X-Retpath-Y";

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadInfoXml {
    pub host: String,
    pub path: String,
    pub ts: String,
    pub s: String,
}

pub fn generate_direct_download_url(xml: &DownloadInfoXml) -> Result<String> {
    let path = xml
        .path
        .strip_prefix('/')
        .context("Unexpected <path> in download-info XML (expected leading '/')")?;
    let sign_target = format!("{}{}{}", DOWNLOAD_SECRET_SALT, path, xml.s);
    let mut hasher = Md5::new();
    hasher.update(sign_target.as_bytes());
    let digest = hasher.finalize();
    let sign_hash = format!("{:x}", digest);

    let url = format!(
        "https://{}/get-mp3/{}/{}{}",
        xml.host, sign_hash, xml.ts, xml.path
    );
    Ok(url)
}

pub fn parse_download_info_xml(xml_content: &str) -> Result<DownloadInfoXml> {
    let host =
        extract_xml_tag(xml_content, "host").context("Missing <host> tag in download-info XML")?;
    let path =
        extract_xml_tag(xml_content, "path").context("Missing <path> tag in download-info XML")?;
    let ts = extract_xml_tag(xml_content, "ts").context("Missing <ts> tag in download-info XML")?;
    let s = extract_xml_tag(xml_content, "s").context("Missing <s> tag in download-info XML")?;

    Ok(DownloadInfoXml { host, path, ts, s })
}

fn extract_xml_tag(content: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{tag}>");
    let close_tag = format!("</{tag}>");
    let start = content.find(&open_tag)? + open_tag.len();
    let end = content[start..].find(&close_tag)? + start;
    Some(content[start..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<download-info><host>s123.storage.yandex.net</host><path>/rmusic/U2FsdGVk/abc</path><ts>0005f1e2d3c4b</ts><region>-1</region><s>deadbeef</s></download-info>"#;

    #[test]
    fn parses_download_info_xml() {
        let xml = parse_download_info_xml(SAMPLE_XML).unwrap();
        assert_eq!(xml.host, "s123.storage.yandex.net");
        assert_eq!(xml.path, "/rmusic/U2FsdGVk/abc");
        assert_eq!(xml.ts, "0005f1e2d3c4b");
        assert_eq!(xml.s, "deadbeef");
    }

    #[test]
    fn missing_tag_is_an_error() {
        assert!(parse_download_info_xml("<download-info><host>h</host></download-info>").is_err());
    }

    #[test]
    fn builds_signed_url() {
        let xml = parse_download_info_xml(SAMPLE_XML).unwrap();
        let expected_sign = format!(
            "{:x}",
            Md5::digest(b"X-Retpath-Yrmusic/U2FsdGVk/abcdeadbeef")
        );
        assert_eq!(
            generate_direct_download_url(&xml).unwrap(),
            format!(
                "https://s123.storage.yandex.net/get-mp3/{expected_sign}/0005f1e2d3c4b/rmusic/U2FsdGVk/abc"
            )
        );
    }

    #[test]
    fn empty_path_does_not_panic() {
        let xml = DownloadInfoXml {
            host: "h".into(),
            path: String::new(),
            ts: "t".into(),
            s: "s".into(),
        };
        assert!(generate_direct_download_url(&xml).is_err());
    }
}

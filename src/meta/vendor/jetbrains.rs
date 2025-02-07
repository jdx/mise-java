use crate::{
    github::{self, GitHubRelease},
    http::HTTP,
    meta::JavaMetaData,
};
use eyre::Result;
use log::{debug, error, warn};
use scraper::{Html, Selector};
use xx::regex;

use super::{md_to_html, normalize_architecture, normalize_os, normalize_version, Vendor};

#[derive(Clone, Copy, Debug)]
pub struct Jetbrains {}

struct FileNameMeta {
    arch: String,
    ext: String,
    image_type: String,
    os: String,
    version: String,
}

impl Vendor for Jetbrains {
    fn get_name(&self) -> String {
        "jetbrains".to_string()
    }

    fn fetch_metadata(&self, meta_data: &mut Vec<crate::meta::JavaMetaData>) -> eyre::Result<()> {
        let releases = github::list_releases("JetBrains/JetBrainsRuntime")?;
        for release in &releases {
            meta_data.extend(map_release(release)?);
        }
        Ok(())
    }
}

fn map_release(release: &GitHubRelease) -> Result<Vec<JavaMetaData>> {
    let mut meta_data = vec![];
    let version = release.tag_name.clone();
    let html = match release.body {
        Some(ref body) => md_to_html(body.as_str()),
        None => {
            warn!("[jetbrains] no body found for release: {version}");
            return Ok(meta_data);
        }
    };
    let fragment = Html::parse_fragment(&html);
    let a_selector =
        Selector::parse("table a:is([href$='.pkg'], [href$='.tar.gz'], [href$='.zip'])").unwrap();
    for a in fragment.select(&a_selector) {
        let href = a
            .value()
            .attr("href")
            .ok_or_else(|| eyre::eyre!("no href found"))?;
        let name = href
            .split("/")
            .last()
            .ok_or_else(|| eyre::eyre!("no name found"))?
            .to_string();
        let filename_meta = meta_from_name(&name)?;
        let features = normalize_features(&name);
        let sha512_url = format!("{}.checksum", &href);
        let sha512 = match HTTP.get_text(&sha512_url) {
            Ok(sha) => sha.split_whitespace().next().map(|s| s.to_string()),
            Err(e) => {
                error!("error fetching sha512sum for {name}: {e}");
                None
            }
        };
        meta_data.push(JavaMetaData {
            architecture: normalize_architecture(&filename_meta.arch),
            features: Some(features),
            filename: name.to_string(),
            file_type: filename_meta.ext,
            image_type: filename_meta.image_type,
            java_version: normalize_version(&filename_meta.version),
            jvm_impl: "hotspot".to_string(),
            os: normalize_os(&filename_meta.os),
            release_type: match release.prerelease {
                true => "ea".to_string(),
                false => "ga".to_string(),
            },
            sha512,
            sha512_url: Some(sha512_url),
            url: href.to_string(),
            version: normalize_version(&filename_meta.version),
            vendor: "jetbrains".to_string(),
            ..Default::default()
        });
    }
    Ok(meta_data)
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[jetbrains] parsing name: {}", name);
    let capture = regex!(r"^jbr(sdk)?(?:_\w+)?-([0-9][0-9\+._]{1,})-(linux-musl|linux|osx|macos|windows)-(aarch64|x64|x86)(?:-\w+)?-(b[0-9\+.]{1,})(?:_\w+)?\.(tar\.gz|zip|pkg)$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression did not match name: {}", name))?;

    let image_type = capture.get(1).map_or("jre", |m| m.as_str()).to_string();
    let os = capture.get(3).unwrap().as_str().to_string();
    let arch = capture.get(4).unwrap().as_str().to_string();
    let version = format!(
        "{}{}",
        capture.get(2).unwrap().as_str().to_string(),
        capture.get(5).unwrap().as_str().to_string()
    );
    let ext = capture.get(6).unwrap().as_str().to_string();

    Ok(FileNameMeta {
        arch,
        ext,
        image_type,
        os,
        version,
    })
}

fn normalize_features(name: &str) -> Vec<String> {
    let mut features = vec![];
    let name = name.to_lowercase();
    if name.contains("_diz") {
        features.push("debug".to_string());
    }
    if name.contains("jcef") {
        features.push("jcef".to_string());
    }
    if name.contains("-fastdebug") {
        features.push("fastdebug".to_string());
    }
    if name.contains("_fd") {
        features.push("jcef".to_string());
        features.push("fastdebug".to_string());
    }
    if name.contains("_ft") {
        features.push("freetype".to_string());
    }
    if name.contains("musl") {
        features.push("musl".to_string());
    }
    features
}

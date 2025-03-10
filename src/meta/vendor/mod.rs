use std::{
    collections::HashSet,
    sync::{Arc, LazyLock},
};

use comrak::{ComrakOptions, markdown_to_html};
use eyre::Result;
use indoc::formatdoc;
use log::info;
use scraper::{Html, Selector};

use super::JavaMetaData;

pub mod corretto;
pub mod dragonwell;
pub mod graalvm;
pub mod jetbrains;
pub mod kona;
pub mod liberica;
pub mod mandrel;
pub mod microsoft;
pub mod openjdk;
pub mod oracle;
pub mod sapmachine;
pub mod temurin;
pub mod zulu;

// TODO: implement all vendors
pub static VENDORS: LazyLock<Vec<Arc<dyn Vendor>>> = LazyLock::new(|| {
    vec![
        Arc::new(corretto::Corretto {}),
        Arc::new(dragonwell::Dragonwell {}),
        Arc::new(graalvm::GraalVM {}),
        Arc::new(jetbrains::Jetbrains {}),
        Arc::new(kona::Kona {}),
        Arc::new(liberica::Liberica {}),
        Arc::new(mandrel::Mandrel {}),
        Arc::new(microsoft::Microsoft {}),
        Arc::new(openjdk::OpenJDK {}),
        Arc::new(oracle::Oracle {}),
        Arc::new(sapmachine::SAPMachine {}),
        Arc::new(temurin::Temurin {}),
        Arc::new(zulu::Zulu {}),
    ]
});

/// Represents a vendor of Java distributions
///
/// A vendor is responsible for fetching the metadata of all available Java versions
///
pub trait Vendor: Send + Sync {
    /// Returns the name of the vendor
    fn get_name(&self) -> String;

    /// Fetches the metadata of all available Java versions for a vendor
    fn fetch(&self) -> Result<HashSet<JavaMetaData>> {
        let mut meta_data = HashSet::new();
        let start = std::time::Instant::now();
        self.fetch_metadata(&mut meta_data)?;

        info!(
            "[{}] fetched {} entries in {:.2} seconds",
            self.get_name(),
            meta_data.len(),
            start.elapsed().as_secs_f32()
        );
        Ok(meta_data)
    }

    /// Fetches the metadata of all available Java versions for a vendor
    fn fetch_metadata(&self, meta_data: &mut HashSet<JavaMetaData>) -> Result<()>;
}

/// An anchor element with a name and href
pub struct AnchorElement {
    name: String,
    href: String,
}

/// Returns the file extension of a package which is either `apk`, `deb`, `dmg`, `msi`, `pkg`, `rpm`, `tar.gz` or `zip`
fn get_extension(package_name: &str) -> String {
    let re = regex::Regex::new(r"^.*\.(apk|dep|dmg|msi|pkg|rpm|tar\.gz|zip)$").unwrap();
    re.replace(package_name, "$1").to_string()
}

/// Returns HTML from a Markdown
pub fn md_to_html(md: &str) -> String {
    let markdown_input = formatdoc! {r#"
  {markdown}
  "#,
      markdown = md.replace("\\r\\n", "\n"),
    };

    let mut options = ComrakOptions::default();
    options.extension.table = true;

    markdown_to_html(&markdown_input, &options)
}

/// Extract anchor elements from HTML
pub fn anchors_from_html(html: &str, selector: &str) -> Vec<AnchorElement> {
    let document = Html::parse_document(html);
    let a_selector = Selector::parse(selector).unwrap();
    document
        .select(&a_selector)
        .map(|a| {
            let name = a.text().collect::<String>();
            let href = a.value().attr("href").unwrap_or("").to_string();
            AnchorElement { name, href }
        })
        .collect::<Vec<AnchorElement>>()
}

/// Normalizes the architecture string to a common format
fn normalize_architecture(architecture: &str) -> String {
    match architecture {
        "amd64" | "x64" | "x86_64" | "x86-64" => "x86_64".to_string(),
        "x32" | "x86" | "x86_32" | "x86-32" | "i386" | "i586" | "i686" => "i686".to_string(),
        "aarch64" | "arm64" => "aarch64".to_string(),
        "arm32" | "armv7" | "arm" | "aarch32sf" => "arm32".to_string(),
        "arm32-vfp-hflt" | "aarch32hf" => "arm32-vfp-hflt".to_string(),
        "ppc64" => "ppc64".to_string(),
        "ppc64le" => "ppc64le".to_string(),
        "s390" => "s390".to_string(),
        "s390x" => "s390x".to_string(),
        "sparcv9" => "sparc".to_string(),
        "riscv64" => "riscv64".to_string(),
        _ => format!("unknown-arch-{architecture}"),
    }
}

/// Normalizes the OS string to a common format
pub fn normalize_os(os: &str) -> String {
    match os.to_lowercase().as_str() {
        "linux" | "alpine" | "alpine-linux" | "linux-musl" | "linux_musl" => "linux".to_string(),
        "mac" | "macos" | "macosx" | "osx" | "darwin" => "macosx".to_string(),
        "win" | "windows" => "windows".to_string(),
        "solaris" => "solaris".to_string(),
        "aix" => "aix".to_string(),
        _ => format!("unknown-os-{os}"),
    }
}

/// Normalizes a major only version string to a semver compatible format
/// Examples:
/// ```plaintext
/// 18 -> 18.0.0
/// 18-beta -> 18.0.0-beta
/// 18+build -> 18.0.0+build
/// ```
pub fn normalize_version(version: &str) -> String {
    let re = regex::Regex::new(r"^([0-9]+)([-+].+)?$").unwrap();
    if let Some(caps) = re.captures(version) {
        let major = caps.get(1).map_or("", |m| m.as_str());
        let suffix = caps.get(2).map_or("", |m| m.as_str());
        if suffix.is_empty() {
            format!("{}.0.0", major)
        } else {
            format!("{}.0.0{}", major, suffix)
        }
    } else {
        version.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_extension() {
        assert_eq!(get_extension("jdk-8u292-linux-x64.apk"), "apk");
        assert_eq!(get_extension("jdk-8u292-macosx-x64.dmg"), "dmg");
        assert_eq!(get_extension("jdk-8u292-windows-x64.msi"), "msi");
        assert_eq!(get_extension("jdk-8u292-linux-x64.pkg"), "pkg");
        assert_eq!(get_extension("jdk-8u292-linux-x64.rpm"), "rpm");
        assert_eq!(get_extension("jdk-8u292-linux-x64.tar.gz"), "tar.gz");
        assert_eq!(get_extension("jdk-8u292-windows-x64.zip"), "zip");
    }

    #[test]
    fn test_normalize_architecture() {
        assert_eq!(normalize_architecture("amd64"), "x86_64");
        assert_eq!(normalize_architecture("x64"), "x86_64");
        assert_eq!(normalize_architecture("x86_64"), "x86_64");
        assert_eq!(normalize_architecture("x86-64"), "x86_64");
        assert_eq!(normalize_architecture("x32"), "i686");
        assert_eq!(normalize_architecture("x86"), "i686");
        assert_eq!(normalize_architecture("x86_32"), "i686");
        assert_eq!(normalize_architecture("x86-32"), "i686");
        assert_eq!(normalize_architecture("i386"), "i686");
        assert_eq!(normalize_architecture("i586"), "i686");
        assert_eq!(normalize_architecture("i686"), "i686");
        assert_eq!(normalize_architecture("aarch64"), "aarch64");
        assert_eq!(normalize_architecture("arm64"), "aarch64");
        assert_eq!(normalize_architecture("arm"), "arm32");
        assert_eq!(normalize_architecture("arm32"), "arm32");
        assert_eq!(normalize_architecture("armv7"), "arm32");
        assert_eq!(normalize_architecture("aarch32sf"), "arm32");
        assert_eq!(normalize_architecture("arm32-vfp-hflt"), "arm32-vfp-hflt");
        assert_eq!(normalize_architecture("aarch32hf"), "arm32-vfp-hflt");
        assert_eq!(normalize_architecture("ppc64"), "ppc64");
        assert_eq!(normalize_architecture("ppc64le"), "ppc64le");
        assert_eq!(normalize_architecture("s390"), "s390");
        assert_eq!(normalize_architecture("s390x"), "s390x");
        assert_eq!(normalize_architecture("sparcv9"), "sparc");
        assert_eq!(normalize_architecture("riscv64"), "riscv64");
        assert_eq!(normalize_architecture("unknown"), "unknown-arch-unknown");
    }

    #[test]
    fn test_normalize_os() {
        assert_eq!(normalize_os("linux"), "linux");
        assert_eq!(normalize_os("alpine"), "linux");
        assert_eq!(normalize_os("alpine-linux"), "linux");
        assert_eq!(normalize_os("mac"), "macosx");
        assert_eq!(normalize_os("macos"), "macosx");
        assert_eq!(normalize_os("macosx"), "macosx");
        assert_eq!(normalize_os("osx"), "macosx");
        assert_eq!(normalize_os("darwin"), "macosx");
        assert_eq!(normalize_os("win"), "windows");
        assert_eq!(normalize_os("windows"), "windows");
        assert_eq!(normalize_os("solaris"), "solaris");
        assert_eq!(normalize_os("aix"), "aix");
        assert_eq!(normalize_os("unknown"), "unknown-os-unknown");
    }

    #[test]
    fn test_normalize_version() {
        assert_eq!(normalize_version("1"), "1.0.0");
        assert_eq!(normalize_version("1-beta"), "1.0.0-beta");
        assert_eq!(normalize_version("1+build"), "1.0.0+build");
        assert_eq!(normalize_version("1.2"), "1.2");
        assert_eq!(normalize_version("1.2.3"), "1.2.3");
        assert_eq!(normalize_version("1.2-beta"), "1.2-beta");
        assert_eq!(normalize_version("1.2+build"), "1.2+build");
        assert_eq!(normalize_version("1.2.3-beta"), "1.2.3-beta");
        assert_eq!(normalize_version("1.2.3+build"), "1.2.3+build");
        assert_eq!(normalize_version("invalid"), "invalid");
    }
}

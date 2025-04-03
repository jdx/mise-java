use std::{
    collections::HashSet,
    sync::{Arc, LazyLock},
};

use comrak::{ComrakOptions, markdown_to_html};
use eyre::Result;
use indoc::formatdoc;
use log::info;
use scraper::{Html, Selector};
use xx::regex;

use super::JvmData;

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
pub mod oracle_graalvm;
pub mod sapmachine;
pub mod semeru;
pub mod temurin;
pub mod trava;
pub mod zulu;

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
        Arc::new(oracle_graalvm::OracleGraalVM {}),
        Arc::new(sapmachine::SAPMachine {}),
        Arc::new(semeru::Semeru {}),
        Arc::new(trava::Trava {}),
        Arc::new(temurin::Temurin {}),
        Arc::new(zulu::Zulu {}),
    ]
});

/// Represents a vendor of Java distributions
///
/// A vendor is responsible for fetching the data of all available Java versions
///
pub trait Vendor: Send + Sync {
    /// Returns the name of the vendor
    fn get_name(&self) -> String;

    /// Fetches the data of all available Java versions for a vendor
    fn fetch(&self) -> Result<HashSet<JvmData>> {
        let mut jvm_data = HashSet::new();
        let start = std::time::Instant::now();
        self.fetch_data(&mut jvm_data)?;

        info!(
            "[{}] fetched {} entries in {:.2} seconds",
            self.get_name(),
            jvm_data.len(),
            start.elapsed().as_secs_f32()
        );
        Ok(jvm_data)
    }

    /// Fetches the data of all available Java versions for a vendor
    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> Result<()>;
}

/// An anchor element with a name and href
pub struct AnchorElement {
    name: String,
    href: String,
}

/// Returns the file extension of a package which is either `apk`, `deb`, `dmg`, `msi`, `pkg`, `rpm`, `tar.gz` or `zip`
fn get_extension(package_name: &str) -> String {
    let re = regex::Regex::new(r"^.*\.(apk|deb|dmg|msi|pkg|rpm|tar\.gz|zip)$").unwrap();
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

#[test]
fn test_anchors_from_html() {
    let html = r#"
  <html>
    <body>
      <a href="https://example.com">Example</a>
      <a href="https://rust-lang.org">Rust</a>
      <a>Missing Href</a>
    </body>
  </html>
  "#;
    let selector = "a";
    let anchors = anchors_from_html(html, selector);

    assert_eq!(anchors.len(), 3);
    for (actual_name, actual_href, expected_name, expected_href) in [
        (
            anchors[0].name.as_str(),
            anchors[0].href.as_str(),
            "Example",
            "https://example.com",
        ),
        (
            anchors[1].name.as_str(),
            anchors[1].href.as_str(),
            "Rust",
            "https://rust-lang.org",
        ),
        (anchors[2].name.as_str(), anchors[2].href.as_str(), "Missing Href", ""),
    ] {
        assert_eq!(actual_name, expected_name);
        assert_eq!(actual_href, expected_href);
    }
}

/// Normalizes the architecture string to a common format
fn normalize_architecture(architecture: &str) -> String {
    match architecture {
        "amd64" | "x64" | "x86_64" | "x86-64" | "x86lx64" => "x86_64".to_string(),
        "x32" | "x86" | "x86_32" | "x86-32" | "i386" | "i586" | "i686" => "i686".to_string(),
        "aarch64" | "arm64" => "aarch64".to_string(),
        "arm32" | "armv7" | "arm" | "aarch32sf" => "arm32".to_string(),
        "arm32-vfp-hflt" | "aarch32hf" => "arm32-vfp-hflt".to_string(),
        "ppc" => "ppc32".to_string(),
        "ppc32hf" => "ppc32hf".to_string(),
        "ppc32spe" => "ppc32spe".to_string(),
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

/// Normalizes a  version string to a semver compatible format
/// Examples:
/// ```plaintext
/// 18-beta -> 18.0.0-beta
/// 18_0_0+build -> 18.0.0+build
/// ```
pub fn normalize_version(version: &str) -> String {
    let version = normalize_major(version);
    normalize_underline(&version)
}

/// Normalizes a major only version string to a semver compatible format
/// Examples:
/// ```plaintext
/// 18 -> 18.0.0
/// 18-beta -> 18.0.0-beta
/// ```
fn normalize_major(version: &str) -> String {
    if let Some(caps) = regex!(r"^([0-9]+)([-+].+)?$").captures(version) {
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

/// Normalizes a version string containing _ instead of .
/// Examples:
/// ```plaintext
/// 18_0_0 -> 18.0.0
/// 18_0_0+build -> 18.0.0+build
/// ```
fn normalize_underline(version: &str) -> String {
    if let Some(caps) = regex!(r"^(([0-9]+_?)+)([-+].+)?$").captures(version) {
        let version_part = caps.get(1).map_or("", |m| m.as_str()).replace('_', ".");
        let suffix = caps.get(3).map_or("", |m| m.as_str());
        format!("{}{}", version_part, suffix)
    } else {
        version.to_string()
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    #[test]
    fn test_md_to_html() {
        let markdown = indoc! {"
        # Title

        This is a **bold** text.
      "};
        let expected_html = indoc! {"
        <h1>Title</h1>
        <p>This is a <strong>bold</strong> text.</p>
      "};
        assert_eq!(md_to_html(markdown), expected_html);

        let markdown_with_table = indoc! {"
        | Header1 | Header2 |
        |---------|---------|
        | Value1  | Value2  |
      "};
        let expected_html_with_table = indoc! {"
        <table>
        <thead>
        <tr>
        <th>Header1</th>
        <th>Header2</th>
        </tr>
        </thead>
        <tbody>
        <tr>
        <td>Value1</td>
        <td>Value2</td>
        </tr>
        </tbody>
        </table>
      "};
        assert_eq!(md_to_html(markdown_with_table), expected_html_with_table);
    }

    #[test]
    fn test_get_extension() {
        for (actual, expected) in [
            ("jdk-8u292-linux-x64.apk", "apk"),
            ("jdk-8u292-linux-x64.deb", "deb"),
            ("jdk-8u292-macosx-x64.dmg", "dmg"),
            ("jdk-8u292-windows-x64.msi", "msi"),
            ("jdk-8u292-linux-x64.pkg", "pkg"),
            ("jdk-8u292-linux-x64.rpm", "rpm"),
            ("jdk-8u292-linux-x64.tar.gz", "tar.gz"),
            ("jdk-8u292-windows-x64.zip", "zip"),
        ] {
            assert_eq!(get_extension(actual), expected);
        }
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
        for (actual, expected) in [
            ("amd64", "x86_64"),
            ("x64", "x86_64"),
            ("x86_64", "x86_64"),
            ("x86-64", "x86_64"),
            ("x32", "i686"),
            ("x86", "i686"),
            ("x86_32", "i686"),
            ("x86-32", "i686"),
            ("i386", "i686"),
            ("i586", "i686"),
            ("i686", "i686"),
            ("aarch64", "aarch64"),
            ("arm64", "aarch64"),
            ("arm", "arm32"),
            ("arm32", "arm32"),
            ("armv7", "arm32"),
            ("aarch32sf", "arm32"),
            ("arm32-vfp-hflt", "arm32-vfp-hflt"),
            ("aarch32hf", "arm32-vfp-hflt"),
            ("ppc", "ppc32"),
            ("ppc32hf", "ppc32hf"),
            ("ppc32spe", "ppc32spe"),
            ("ppc64", "ppc64"),
            ("ppc64le", "ppc64le"),
            ("s390", "s390"),
            ("s390x", "s390x"),
            ("sparcv9", "sparc"),
            ("riscv64", "riscv64"),
        ] {
            assert_eq!(normalize_architecture(actual), expected);
        }
    }

    #[test]
    fn test_normalize_os() {
        for (actual, expected) in [
            ("linux", "linux"),
            ("alpine", "linux"),
            ("alpine-linux", "linux"),
            ("linux-musl", "linux"),
            ("linux_musl", "linux"),
            ("mac", "macosx"),
            ("macos", "macosx"),
            ("macosx", "macosx"),
            ("osx", "macosx"),
            ("darwin", "macosx"),
            ("win", "windows"),
            ("windows", "windows"),
            ("solaris", "solaris"),
            ("aix", "aix"),
            ("unknown", "unknown-os-unknown"),
        ] {
            assert_eq!(normalize_os(actual), expected);
        }
    }

    #[test]
    fn test_normalize_version() {
        for (actual, expected) in [
            ("1", "1.0.0"),
            ("1-beta", "1.0.0-beta"),
            ("1+build", "1.0.0+build"),
            ("1.2", "1.2"),
            ("1.2.3", "1.2.3"),
            ("1.2-beta", "1.2-beta"),
            ("1.2+build", "1.2+build"),
            ("1.2.3-beta", "1.2.3-beta"),
            ("1.2.3+build", "1.2.3+build"),
            ("1_2_3-build", "1.2.3-build"),
            ("invalid", "invalid"),
        ] {
            assert_eq!(normalize_version(actual), expected);
        }
    }
}

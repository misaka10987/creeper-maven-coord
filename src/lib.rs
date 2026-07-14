use std::{
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, bail, ensure};
use semver::Version;
use tracing::warn;

/// Check whether the given string is a valid domain name for a java package.
pub fn is_valid_java_package(name: &str) -> bool {
    let name = name.replace("-", "_");

    for piece in name.split(".") {
        let mut chars = piece.chars();

        if chars.next().is_none_or(|c| !c.is_ascii_lowercase()) {
            return false;
        }

        if !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return false;
        }
    }

    true
}

/// Check whether the given string is a valid [maven artifact identifier](https://maven.apache.org/guides/mini/guide-naming-conventions.html#artifact-identifier), i.e. consists only of lowercase ascii letters, digits and hyphens.
pub fn is_valid_maven_artifact(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// A maven coordinate defined as `<group>:<artifact>:<version>[:<classifier>][@<extension>]`.
///
/// # Examples
///
/// ```
/// use creeper_maven_coord::MavenCoord;
///
/// const COORD: &str = "net.neoforged:neoform:1.21.1-20240808.144430:mappings-merged@txt";
/// const PATH: &str = "net/neoforged/neoform/1.21.1-20240808.144430/neoform-1.21.1-20240808.144430-mappings-merged.txt";
///
/// let x = MavenCoord::new(
///     "net.neoforged".into(),
///     "neoform".into(),
///     "1.21.1-20240808.144430".into(),
///     Some("mappings-merged".into()),
///     Some("txt".into()),
/// )
/// .unwrap();
///
/// let y: MavenCoord = COORD.parse::<MavenCoord>().unwrap();
/// let z = MavenCoord::from_path(PATH).unwrap();
///
/// assert_eq!(x, y);
/// assert_eq!(x, z);
///
/// assert_eq!(y.path(), *PATH);
/// assert_eq!(z.to_string(), COORD);
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct MavenCoord {
    /// The [Group ID](https://maven.apache.org/pom.html#maven-coordinates).
    ///
    /// # Note
    ///
    /// The bahavior is undefined unless this is a valid java package name, i.e. [`is_valid_java_package`].
    pub group: String,

    /// The [Artifact ID](https://maven.apache.org/pom.html#maven-coordinates).
    pub artifact: String,

    /// The [version](https://maven.apache.org/pom.html#maven-coordinates).
    ///
    /// This is encourged, but not guaranteed to be, a [Semantic Version](https://semver.org/).
    pub version: String,

    /// The path extension, defaults to `"jar"` if not specified.
    pub extension: Option<String>,

    /// The classifier.
    pub classifier: Option<String>,
}

impl MavenCoord {
    /// Construct a new maven coordinate, checking validity of fields.
    pub fn new(
        group: String,
        artifact: String,
        version: String,
        classifier: Option<String>,
        extension: Option<String>,
    ) -> anyhow::Result<Self> {
        if !is_valid_java_package(&group) {
            bail!("invalid group {group} in maven coordinate");
        }

        let value = Self {
            group,
            artifact,
            version,
            classifier,
            extension,
        };

        if !is_valid_maven_artifact(&value.artifact) {
            // does only warn because neoforge uses CamelCase in some artifact names
            warn!("invalid artifact name {} in {value}", value.artifact);
        }

        if !value.version.parse::<Version>().is_ok() {
            warn!(
                "unencouraged non-semver version {} in {value} ",
                value.version
            );
        }

        Ok(value)
    }

    /// Storage path of this coordinate according to [Maven Repository Layout](https://maven.apache.org/repositories/layout.html).
    pub fn path(&self) -> PathBuf {
        let classifier = if let Some(s) = &self.classifier {
            &format!("-{s}")
        } else {
            ""
        };

        let extension = if let Some(s) = &self.extension {
            &format!(".{s}")
        } else {
            ".jar"
        };

        let name = format!("{}-{}{classifier}{extension}", self.artifact, self.version);

        self.group
            .split(".")
            .fold(PathBuf::new(), |acc, x| acc.join(x))
            .join(&self.artifact)
            .join(self.version.to_string())
            .join(name)
    }

    /// Try to retrieve a maven coordinate from a path following [Maven Repository Layout](https://maven.apache.org/repositories/layout.html).
    pub fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let mut path = path.as_ref().to_path_buf();

        let extension = if let Some(s) = path.extension() {
            let name = s
                .to_str()
                .ok_or(anyhow!("invalid maven path extension {}", s.display()))?;

            if name == "jar" {
                None
            } else {
                Some(name.into())
            }
        } else {
            Some("".into())
        };

        path.set_extension("");

        let mut components = vec![];

        for c in path.components() {
            let name = c.as_os_str();

            let name = name
                .to_str()
                .ok_or(anyhow!("invalid maven path component {}", name.display()))?;
            components.push(name);
        }

        ensure!(
            components.len() >= 4,
            "invalid maven path {}, expected at least 4 components",
            path.display()
        );

        let last = components[components.len() - 1];
        let version = components[components.len() - 2];
        let artifact = components[components.len() - 3];

        let classifier = last
            .strip_prefix(&format!("{artifact}-{version}"))
            .ok_or(anyhow!("invalid maven path {}", path.display()))?;

        let classifier = if classifier.is_empty() {
            None
        } else {
            Some(
                classifier
                    .strip_prefix("-")
                    .ok_or(anyhow!("invalid classifier {classifier} in maven path"))?,
            )
        };

        let group = components[..components.len() - 3].join(".");

        Ok(Self::new(
            group,
            artifact.into(),
            version.into(),
            classifier.map(|s| s.into()),
            extension,
        )?)
    }
}

impl Display for MavenCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let classifier = if let Some(s) = &self.classifier {
            &format!(":{s}")
        } else {
            ""
        };

        let extension = if let Some(s) = &self.extension {
            &format!("@{s}")
        } else {
            ""
        };

        write!(
            f,
            "{}:{}:{}{classifier}{extension}",
            self.group, self.artifact, self.version
        )
    }
}

impl FromStr for MavenCoord {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pieces = s.split("@").collect::<Vec<_>>();

        let (main, extension) = match pieces.len() {
            0 => unreachable!(),
            1 => (pieces[0], None),
            2 => (pieces[0], Some(pieces[1])),
            _ => bail!(
                "invalid maven coordinate {s}, expected <group>:<artifact>:<version>[:<classifier>][@<extension>]"
            ),
        };

        let pieces = main.split(":").collect::<Vec<_>>();

        let (group, artifact, version, classifier) = match pieces.len() {
            0 => unreachable!(),
            3 => (pieces[0], pieces[1], pieces[2], None),
            4 => (pieces[0], pieces[1], pieces[2], Some(pieces[3])),
            _ => bail!(
                "invalid maven coordinate {s}, expected <group>:<artifact>:<version>[:<classifier>][@<extension>]"
            ),
        };

        Ok(Self::new(
            group.into(),
            artifact.into(),
            version.into(),
            classifier.map(|s| s.into()),
            extension.map(|s| s.into()),
        )?)
    }
}

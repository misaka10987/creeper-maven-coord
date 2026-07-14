# creeper-maven-coord

A Rust library for processing [Maven Coordinates](https://maven.apache.org/pom.html#maven-coordinates) .

## Usage

```rust
use creeper_maven_coord::MavenCoord;

const COORD: &str = "net.neoforged:neoform:1.21.1-20240808.144430:mappings-merged@txt";
const PATH: &str = "net/neoforged/neoform/1.21.1-20240808.144430/neoform-1.21.1-20240808.144430-mappings-merged.txt";

let x = MavenCoord::new(
    "net.neoforged".into(),
    "neoform".into(),
    "1.21.1-20240808.144430".into(),
    Some("mappings-merged".into()),
    Some("txt".into()),
)
.unwrap();

let y: MavenCoord = COORD.parse::<MavenCoord>().unwrap();
let z = MavenCoord::from_path(PATH).unwrap();

assert_eq!(x, y);
assert_eq!(x, z);

assert_eq!(y.path(), *PATH);
assert_eq!(z.to_string(), COORD);
```

## Note

This library is specially tailored for use in Minecraft-related projects. Though unlikely, behavioral inconsistency might exist with the Maven standard.

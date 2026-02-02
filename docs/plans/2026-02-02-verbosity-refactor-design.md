# Design: Refonte du système de verbosité

**Date**: 2026-02-02
**Status**: Ready for implementation
**Supersedes**: `2026-02-02-verbosity-control-design.md`

## Objectif

Corriger et améliorer le système de verbosité existant pour avoir trois modes bien distincts avec des comportements clairs.

## Comportement des trois modes

### Mode Quiet (`-q`)

Output uniquement le(s) chemin(s) des fichiers générés, un par ligne.

```
./output/layer1.png
./output/layer2.png
```

- Aucun message de progression
- Aucun timing
- Les erreurs vont toujours sur stderr

### Mode Normal (défaut)

Messages de progression essentiels sans préfixe + barres de progression.

```
Reading GeoJSON file...
Found 150 geometries
Rendering 1024x768 image...
[##########>---------] 75/150 (50%)
Saved output.png
```

### Mode Verbose (`-v`)

Logs détaillés avec :
- Timestamps elapsed à gauche `[0.00s]`
- Niveaux colorés `[INFO]`, `[DEBUG]`, `[WARN]`, `[ERROR]`
- Une ligne par géométrie traitée
- Récap des timings à la fin
- PAS de barres de progression

```
[0.00s] [INFO] Reading GeoJSON file...
[0.02s] [INFO] Found 150 geometries
[0.03s] [DEBUG] Resolution: 0.0001 degrees/pixel
[0.03s] [INFO] Rendering 1024x768 image...
[0.03s] [DEBUG] Rendering geometry 1/150
[0.04s] [DEBUG] Rendering geometry 2/150
...
[0.18s] [DEBUG] Rendering geometry 150/150
[0.19s] [INFO] Saved output.png
[0.19s] [DEBUG] Timings: read=23ms, render=156ms, save=12ms, total=191ms
```

## Couleurs (mode verbose uniquement)

| Niveau | Couleur | Code ANSI |
|--------|---------|-----------|
| ERROR | Rouge | `\x1b[31m` |
| WARN | Jaune | `\x1b[33m` |
| INFO | Bleu | `\x1b[34m` |
| DEBUG | Gris | `\x1b[90m` |
| Timestamp | Gris dim | `\x1b[90m` |

### Détection automatique des couleurs

1. Si `--no-color` flag → désactivé
2. Sinon si variable env `NO_COLOR` existe → désactivé
3. Sinon si stdout n'est pas un TTY → désactivé
4. Sinon → activé

## Refonte de `logger.rs`

### Nouvelles fonctionnalités

```rust
use std::sync::OnceLock;
use std::time::Instant;

static LOGGER: OnceLock<Logger> = OnceLock::new();
static START_TIME: OnceLock<Instant> = OnceLock::new();

pub struct Logger {
    level: VerbosityLevel,
    colors_enabled: bool,
}

impl Logger {
    pub fn init(level: VerbosityLevel, no_color: bool) {
        let colors_enabled = !no_color
            && std::env::var("NO_COLOR").is_err()
            && atty::is(atty::Stream::Stdout);

        START_TIME.set(Instant::now()).ok();
        LOGGER.set(Logger { level, colors_enabled }).expect("Logger already initialized");
    }

    fn elapsed(&self) -> f64 {
        START_TIME.get().map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0)
    }

    // Nouvelle fonction pour output quiet (juste le path)
    pub fn output(&self, path: &str) {
        match self.level {
            VerbosityLevel::Quiet => println!("{}", path),
            VerbosityLevel::Normal => println!("{}", path),
            VerbosityLevel::Verbose => {
                self.log_with_level("INFO", path);
            }
        }
    }

    // Nouvelle fonction warn
    pub fn warn(&self, msg: &str) {
        match self.level {
            VerbosityLevel::Quiet => {} // silent
            VerbosityLevel::Normal => println!("{}", msg),
            VerbosityLevel::Verbose => self.log_with_level("WARN", msg),
        }
    }

    pub fn info(&self, msg: &str) {
        match self.level {
            VerbosityLevel::Quiet => {} // silent
            VerbosityLevel::Normal => println!("{}", msg),
            VerbosityLevel::Verbose => self.log_with_level("INFO", msg),
        }
    }

    pub fn debug(&self, msg: &str) {
        if self.level == VerbosityLevel::Verbose {
            self.log_with_level("DEBUG", msg);
        }
    }

    pub fn error(&self, msg: &str) {
        match self.level {
            VerbosityLevel::Verbose => {
                // Avec format verbose sur stderr
                let elapsed = self.elapsed();
                if self.colors_enabled {
                    eprintln!("\x1b[90m[{:.2}s]\x1b[0m \x1b[31m[ERROR]\x1b[0m {}", elapsed, msg);
                } else {
                    eprintln!("[{:.2}s] [ERROR] {}", elapsed, msg);
                }
            }
            _ => eprintln!("Error: {}", msg),
        }
    }

    fn log_with_level(&self, level: &str, msg: &str) {
        let elapsed = self.elapsed();
        let (level_color, reset) = if self.colors_enabled {
            let color = match level {
                "ERROR" => "\x1b[31m",
                "WARN" => "\x1b[33m",
                "INFO" => "\x1b[34m",
                "DEBUG" => "\x1b[90m",
                _ => "",
            };
            (color, "\x1b[0m")
        } else {
            ("", "")
        };

        let time_color = if self.colors_enabled { "\x1b[90m" } else { "" };

        println!("{}[{:.2}s]{} {}[{}]{} {}",
            time_color, elapsed, reset,
            level_color, level, reset,
            msg);
    }
}
```

### Format par mode

| Mode | `info("msg")` | `debug("msg")` | `output("path")` |
|------|---------------|----------------|------------------|
| Quiet | rien | rien | `path` (stdout) |
| Normal | `msg` | rien | `msg` |
| Verbose | `[0.00s] [INFO] msg` | `[0.00s] [DEBUG] msg` | `[0.00s] [INFO] msg` |

## Modification CLI (`cli.rs`)

### Nouveau flag

```rust
/// Disable colored output (auto-detected by default)
#[arg(long)]
pub no_color: bool,
```

### Config mis à jour

```rust
pub struct Config {
    // ... existing fields
    pub verbosity: VerbosityLevel,
    pub no_color: bool,
}
```

## Audit des appels à modifier (`main.rs`)

### Appels à remplacer par `output()`

| Ligne | Actuel | Nouveau |
|-------|--------|---------|
| 295 | `logger::success(&format!("Saved: {}", path))` | `logger::output(&path.display().to_string())` |
| 397 | `logger::success(&format!("Saved: {}", path))` | `logger::output(&path.display().to_string())` |

### Appels à remplacer par `warn()`

| Ligne | Actuel | Nouveau |
|-------|--------|---------|
| 70 | `eprintln!("Warning: No polygon layers...")` | `logger::warn("No polygon layers found in the GeoPackage")` |

### Doublons à supprimer

| Ligne | Contenu | Raison |
|-------|---------|--------|
| 313 | `logger::debug("GeoJSON contains {} geometries")` | Doublon de ligne 312 |
| 343-344 | `logger::debug(resolution, bbox)` | Doublon de 334-338 |
| 359 | `logger::debug("Image dimensions")` | Doublon de 358 |

### Timings à déplacer

| Ligne | Actuel | Nouveau comportement |
|-------|--------|---------------------|
| 187 | `logger::success("Total time: ...")` | Quiet: rien, Normal: `info()`, Verbose: dans récap |
| 396 | `logger::success("Total time: ...")` | Idem |

### Progress bars

- Normal mode: garder les barres `indicatif`
- Verbose mode: remplacer par logs détaillés
- Quiet mode: aucune barre

### Logs par géométrie (verbose only)

Ajouter dans les boucles de rendu (lignes 267-272 et 377-382) :

```rust
for (i, geom) in geometries.iter().enumerate() {
    logger::debug(&format!("Rendering geometry {}/{}", i + 1, total));
    renderer.render_multipolygon(geom);
}
```

## Fichiers à modifier

| Fichier | Changements |
|---------|-------------|
| `src/logger.rs` | Refonte complète : timer global, couleurs, `output()`, `warn()` |
| `src/cli.rs` | Ajouter `--no-color`, passer au Config |
| `src/main.rs` | Adapter appels, conditionner progress bars, ajouter logs géométrie |
| `Cargo.toml` | Ajouter dépendance `atty` |

## Dépendance à ajouter

```toml
[dependencies]
atty = "0.2"
```

## Plan d'implémentation

1. **Ajouter dépendance `atty`** dans Cargo.toml
2. **Modifier `cli.rs`** : ajouter `--no-color` flag
3. **Refondre `logger.rs`** : timer, couleurs, nouvelles fonctions
4. **Mettre à jour `main.rs`** :
   - Remplacer `success("Saved:")` par `output()`
   - Remplacer `eprintln!("Warning:")` par `warn()`
   - Supprimer doublons
   - Conditionner progress bars sur mode
   - Ajouter logs par géométrie en verbose
   - Ajuster les timings finaux
5. **Tests** : adapter les tests existants et ajouter nouveaux tests

## Tests

### Tests unitaires (`logger.rs`)

- Test format quiet : output produit juste le path
- Test format normal : info produit message sans préfixe
- Test format verbose : info produit `[X.XXs] [INFO] msg`
- Test couleurs désactivées avec `--no-color`
- Test warn() à chaque niveau

### Tests CLI (`cli.rs`)

- Test `--no-color` seul
- Test `--no-color` avec `-v`
- Test `--no-color` avec `-q`

### Tests d'intégration

- Vérifier que quiet produit uniquement les paths
- Vérifier que verbose inclut les timestamps

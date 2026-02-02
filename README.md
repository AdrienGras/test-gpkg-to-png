# 🦀 gpkg-to-png 🖼️

[![Rust](https://img.shields.io/badge/rust-v1.70+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Claude Code](https://img.shields.io/badge/Built%20with-Claude%20Code-blue.svg)](https://claude.ai/code)

> Un outil CLI ultra-rapide en Rust pour transformer vos fichiers GeoPackage et GeoJSON en superbes overlays PNG transparents. 🚀

---

## ✨ Fonctionnalités

- 📦 **Formats Multiples** : Supporte les fichiers GeoPackage (`.gpkg`) et GeoJSON (`.geojson`).
- 📚 **Lecture Multi-Couches** : Extrait automatiquement les polygones/multipolygones (GPKG) ou géométries (GeoJSON).
- 🌍 **Reprojection à la volée** : Conversion automatique vers WGS84 (EPSG:4326) avec `proj` pour les GPKG.
- 🎨 **Stylisation Flexible** : Couleurs de remplissage (RGBA) et de contour (RGB) entièrement configurables.
- 📐 **Haute Précision** : Résolution personnalisable en degrés par pixel ou échelle en mètres par pixel.
- 🏎️ **Performance Rust** : Rendu parallélisé pour une vitesse d'exécution optimale.

## 🚀 Installation

### 📋 Prérequis
- [Rust](https://www.rust-lang.org/tools/install) (édition 2021)
- Cargo

### 🏗️ Compilation
```bash
git clone https://github.com/AdrienGras/test-gpkg-to-png.git
cd test-gpkg-to-png
cargo build --release
```
L'exécutable sera disponible dans `target/release/gpkg-to-png`.

> 💡 **Tip** : Vous pouvez également télécharger les binaires pré-compilés pour Linux dans la section [Releases](https://github.com/AdrienGras/test-gpkg-to-png/releases) de ce dépôt.

## 🛠️ Utilisation

```bash
gpkg-to-png <INPUT> [OPTIONS]
```

### ⚙️ Options principales

| Option           | Raccourci | Description                                                             | Défaut                    |
| :--------------- | :-------- | :---------------------------------------------------------------------- | :------------------------ |
| `<INPUT>`        |           | **Argument** : Chemin vers le fichier `.gpkg` ou `.geojson`             |                           |
| `--format`       | `-f`      | Format d'entrée: `gpkg` ou `geojson`                                    | **Requis**                |
| `--verbose`      | `-v`      | Mode verbeux avec timestamps et logs colorés                            |                           |
| `--quiet`        | `-q`      | Mode silencieux (affiche uniquement les chemins des fichiers générés)   |                           |
| `--no-color`     |           | Désactive les couleurs ANSI (auto-détecté si non-TTY)                   |                           |
| `--output-dir`   | `-o`      | Répertoire de sortie                                                    | `.`                       |
| `--bbox`         | `-b`      | Bounding box: `minLon,minLat,maxLon,maxLat`                             | *Auto-détecté si omis*    |
| `--resolution`   | `-r`      | Taille du pixel en degrés (mutuellement exclusif avec `--scale`)        |                           |
| `--scale`        | `-s`      | Échelle en mètres par pixel (mutuellement exclusif avec `--resolution`) |                           |
| `--fill`         |           | Couleur de remplissage RGBA hex (ex: `FF000080`)                        | `FF000080`                |
| `--stroke`       |           | Couleur de contour RGB hex (ex: `FF0000`)                               | `FF0000`                  |
| `--stroke-width` |           | Épaisseur du contour en pixels                                          | `1`                       |
| `--layer`        | `-l`      | Nom de la couche spécifique à rendre (GPKG uniquement)                  | *Toutes*                  |
| `--output-name`  |           | Nom du fichier PNG de sortie (GeoJSON uniquement)                       | *Nom du fichier d'entrée* |
| `--help`         | `-h`      | Afficher l'aide                                                         |                           |
| `--version`      | `-V`      | Afficher la version                                                     |                           |

> **Note** : Vous devez spécifier soit `--resolution`, soit `--scale`. Si la `bbox` n'est pas fournie, l'outil l'auto-détectera à partir de l'emprise des données.

### 💡 Exemples

**Rendu d'un GeoPackage avec couleurs personnalisées :**
```bash
gpkg-to-png zones.gpkg \
  -f gpkg \
  --bbox "-4.5,48.0,-4.0,48.5" \
  --resolution 0.0001 \
  --fill "00FF0080" \
  --stroke "00FF00" \
  --stroke-width 2 \
  -o ./output/
```

**Rendu d'un GeoJSON avec résolution automatique :**
```bash
gpkg-to-png data.geojson \
  -f geojson \
  --scale 10 \
  --output-name "mon-overlay" \
  -o ./output/
```

**Rendu d'une couche spécifique dans un GPKG :**
```bash
gpkg-to-png zones.gpkg \
  -f gpkg \
  --layer "parcelles" \
  --resolution 0.0001 \
  -o ./output/
```

**Mode verbeux avec timestamps détaillés :**
```bash
gpkg-to-png zones.gpkg \
  -f gpkg \
  -v \
  --resolution 0.0001 \
  -o ./output/
# Output: [0.00s] [INFO] Auto-detecting bounding box...
#         [0.02s] [DEBUG] Rendering geometry 1/100
#         ...
```

**Mode silencieux (pour scripts) :**
```bash
gpkg-to-png zones.gpkg -f gpkg -q --resolution 0.0001 -o ./output/
# Output: ./output/zones.png
```

## 🏗️ Architecture du projet

```text
src/
├── main.rs       // 🏗️ Point d'entrée & dispatch par format
├── cli.rs        // ⌨️ Parsing des arguments avec clap
├── gpkg.rs       // 📂 Lecture GeoPackage & reprojection
├── geojson.rs    // 🌐 Lecture GeoJSON (WGS84)
├── render.rs     // 🎨 Algorithmes de rendu (Scanline/Bresenham)
├── render/
│   └── edge.rs   // 📊 Gestion des tables de scanline
├── math.rs       // 📐 Transformations de coordonnées
└── error.rs      // 🚨 Gestion d'erreurs robuste
```

## 🛠️ Dépendances

Le projet utilise les meilleurs outils de l'écosystème Rust :
- `sqlx` & `tokio` pour l'accès aux données asynchrone.
- `geo` & `proj` pour la manipulation géospatiale.
- `geojson` pour le parsing GeoJSON.
- `image` pour le rendu raster haute performance.
- `rayon` pour le parallélisme massif.
- `atty` pour la détection TTY (couleurs automatiques).

## 🧪 Tests

```bash
cargo test                 # ✅ Tests unitaires (48 tests)
cargo test --test integration -- --ignored # 🔍 Tests d'intégration GPKG
cargo test --test geojson_integration -- --ignored # 🌐 Tests d'intégration GeoJSON
```

---

## 📜 Licence

MIT © [Adrien Gras](https://github.com/AdrienGras)

---

## 🧪 À propos de ce POC : La démarche "Vibe Coding"

Ce projet n'est pas qu'un simple outil technique, c'est une **preuve de concept** explorant une nouvelle manière de concevoir du logiciel : le **Vibe Coding**.

L'objectif était de tester la productivité et la pertinence d'une stack de développement 100% assistée par intelligence artificielle de bout en bout.

### 🛠️ Stack de développement utilisée :
- **Orchestration & Exécution** : [Claude Code](https://claude.ai/code) (l'agent CLI qui a écrit ces lignes).
- **Intelligence & "Vibes"** : Un mix dynamique via **OpenRouter**, exploitant principalement les modèles **Claude 4.5 Sonnet** (Anthropic) et **Gemini 3 Flash** (Google).
- **Processus** : Aucun code n'a été écrit à la main. Chaque fonctionnalité, du choix de l'algorithme scanline pour le remplissage à la gestion du parallélisme avec `rayon`, a été proposée, discutée et implémentée par l'IA sous la supervision de l'utilisateur.

### 📊 Bilan de l'expérience :
- ⏱️ **Temps total** : Environ **5 heures**, incluant la conception, l'implémentation, le débogage et la documentation.
- 💰 **Coût** : Environ **60€** de tokens API (OpenRouter / Anthropic).
- ✅ **Résultat** : Un code Rust robuste, typé, performant et entièrement documenté.

*Ce projet démontre qu'avec les bons outils d'IA et une vision claire, on peut transformer une idée en un outil viable en un temps record.* 🚀

# 🦀 gpkg-to-png 🖼️

[![Rust](https://img.shields.io/badge/rust-v1.70+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Claude Code](https://img.shields.io/badge/Built%20with-Claude%20Code-blue.svg)](https://claude.ai/code)

> Un outil CLI ultra-rapide en Rust pour transformer vos fichiers GeoPackage en superbes overlays PNG transparents. 🚀

---

## 🧪 Le Vibe Coding POC

Ce projet est une preuve de concept (POC) réalisée pour tester les limites du **"vibe coding"**. L'intégralité du développement a été orchestrée via **Claude Code**, en exploitant la puissance combinée d'**OpenRouter**, **Claude AI** et **Gemini AI**.

📊 **Quelques chiffres :**
- ⚙️ **Méthode :** 100% assistée par IA (Coding with vibes).
- ⏱️ **Temps de développement :** ~2 heures (du design à la documentation complète).
- 💰 **Coût total :** ~30€ de crédits API.

---

## ✨ Fonctionnalités

- 📦 **Lecture Multi-Couches** : Extrait automatiquement les polygones/multipolygones de vos fichiers `.gpkg`.
- 🌍 **Reprojection à la volée** : Conversion automatique vers WGS84 (EPSG:4326) avec `proj`.
- 🎨 **Stylisation Flexible** : Couleurs de remplissage (RGBA) et de contour (RGB) entièrement configurables.
- 📐 **Haute Précision** : Résolution personnalisable en degrés par pixel.
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

## 🛠️ Utilisation

```bash
gpkg-to-png <INPUT> [OPTIONS]
```

### ⚙️ Options principales

| Option | Raccourci | Description | Défaut |
|:-------|:----------|:------------|:-------|
| `--output-dir` | `-o` | Répertoire de sortie | `.` |
| `--bbox` | `-b` | Bounding box: `minLon,minLat,maxLon,maxLat` | **Requis** |
| `--resolution` | `-r` | Taille du pixel en degrés | **Requis** |
| `--fill` | | Couleur de remplissage RGBA (hex) | `FF000080` |
| `--stroke` | | Couleur de contour RGB (hex) | `FF0000` |
| `--stroke-width`| | Épaisseur du contour | `1` |
| `--layer` | `-l` | Couche spécifique à rendre | Toutes |

### 💡 Exemples

**Rendu avec couleurs personnalisées :**
```bash
gpkg-to-png zones.gpkg \
  --bbox "-4.5,48.0,-4.0,48.5" \
  --resolution 0.0001 \
  --fill "00FF0080" \
  --stroke "00FF00" \
  --stroke-width 2 \
  -o ./output/
```

## 🏗️ Architecture du projet

```text
src/
├── main.rs       // 🏗️ Point d'entrée & pipeline async
├── cli.rs        // ⌨️ Parsing des arguments avec clap
├── gpkg.rs       // 📂 Lecture GeoPackage & reprojection
├── render.rs     // 🎨 Algorithmes de rendu (Scanline/Bresenham)
├── math.rs       // 📐 Transformations de coordonnées
└── error.rs      // 🚨 Gestion d'erreurs robuste
```

## 🛠️ Dépendances

Le projet utilise les meilleurs outils de l'écosystème Rust :
- `sqlx` & `tokio` pour l'accès aux données asynchrone.
- `geo` & `proj` pour la manipulation géospatiale.
- `image` pour le rendu raster haute performance.
- `rayon` pour le parallélisme massif.

## 🧪 Tests

```bash
cargo test                 # ✅ Tests unitaires
cargo test --test integration -- --ignored # 🔍 Tests d'intégration (requiert un .gpkg)
```

---

## 📜 Licence

MIT © [Adrien Gras](https://github.com/AdrienGras)

---
*Généré avec ❤️ par Claude Code et le Vibe Coding.*

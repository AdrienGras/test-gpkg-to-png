# gpkg-to-png

Outil CLI en Rust pour convertir les couches polygones d'un fichier GeoPackage (.gpkg) en images PNG transparentes, idéales pour la superposition cartographique.

## Fonctionnalités

- Lecture des couches polygones/multipolygones depuis un fichier GeoPackage
- Reprojection automatique vers WGS84 (EPSG:4326)
- Rendu avec couleur de remplissage (RGBA) et contour (RGB) configurables
- Résolution configurable en degrés par pixel
- Export PNG avec transparence
- Support des polygones avec trous

## Installation

### Prérequis

- Rust 1.70+ (édition 2021)
- Cargo

### Compilation

```bash
git clone https://github.com/AdrienGras/test-gpkg-to-png.git
cd test-gpkg-to-png
cargo build --release
```

L'exécutable sera disponible dans `target/release/gpkg-to-png`.

## Utilisation

```bash
gpkg-to-png <INPUT> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<INPUT>` | Chemin vers le fichier .gpkg |

### Options

| Option | Description | Défaut |
|--------|-------------|--------|
| `-o, --output-dir <DIR>` | Répertoire de sortie | `.` |
| `-b, --bbox <BBOX>` | Bounding box : `minLon,minLat,maxLon,maxLat` | **Requis** |
| `-r, --resolution <RES>` | Taille du pixel en degrés | **Requis** |
| `--fill <COLOR>` | Couleur de remplissage RGBA (hex) | `FF000080` |
| `--stroke <COLOR>` | Couleur de contour RGB (hex) | `FF0000` |
| `--stroke-width <WIDTH>` | Épaisseur du contour en pixels | `1` |
| `-l, --layer <NAME>` | Couche spécifique à rendre | Toutes |
| `-h, --help` | Afficher l'aide | |
| `-V, --version` | Afficher la version | |

### Exemples

**Rendu basique avec les paramètres par défaut :**

```bash
gpkg-to-png data.gpkg \
  --bbox="-4.8,48.2,-4.3,48.6" \
  --resolution 0.0005
```

**Rendu personnalisé avec couleurs et sortie spécifiées :**

```bash
gpkg-to-png zones.gpkg \
  --bbox="-4.5,48.0,-4.0,48.5" \
  --resolution 0.0001 \
  --fill "00FF0080" \
  --stroke "00FF00" \
  --stroke-width 2 \
  -o ./output/
```

**Rendu d'une seule couche :**

```bash
gpkg-to-png multi_layer.gpkg \
  --bbox="2.0,48.5,2.5,49.0" \
  --resolution 0.0002 \
  --layer "zones_urbaines"
```

### Format des couleurs

- **RGBA** (remplissage) : 8 caractères hexadécimaux, ex: `FF000080` (rouge semi-transparent)
- **RGB** (contour) : 6 caractères hexadécimaux, ex: `00FF00` (vert)

### Calcul des dimensions

L'image de sortie aura les dimensions suivantes :

```
largeur = ceil((maxLon - minLon) / resolution)
hauteur = ceil((maxLat - minLat) / resolution)
```

**Exemple :** Une bbox de 0.5° × 0.4° avec une résolution de 0.0005° produira une image de 1000 × 800 pixels.

## Architecture

```
src/
├── main.rs       # Point d'entrée, pipeline async
├── cli.rs        # Parsing des arguments avec clap
├── error.rs      # Types d'erreurs avec thiserror
├── gpkg.rs       # Lecture GeoPackage et reprojection
├── math.rs       # Transformations de coordonnées
└── render.rs     # Rastérisation des polygones
```

### Pipeline de traitement

1. **Parsing CLI** - Validation des arguments (bbox, couleurs, résolution)
2. **Ouverture GPKG** - Connexion SQLite en lecture seule
3. **Liste des couches** - Identification des couches polygones
4. **Lecture WKB** - Extraction des géométries au format WKB
5. **Reprojection** - Transformation vers WGS84 avec proj
6. **Rastérisation** - Remplissage par scanline + contour Bresenham
7. **Export PNG** - Sauvegarde avec transparence alpha

## Dépendances

| Crate | Version | Usage |
|-------|---------|-------|
| `clap` | 4 | Parsing CLI |
| `geo` | 0.28 | Types géométriques |
| `image` | 0.25 | Création d'images |
| `proj` | 0.31 | Reprojection CRS |
| `sqlx` | 0.8 | Accès SQLite/GeoPackage |
| `tokio` | 1 | Runtime async |
| `thiserror` | 1 | Gestion d'erreurs |
| `wkb` | 0.7 | Parsing WKB |
| `hex` | 0.4 | Parsing couleurs |

## Tests

```bash
# Tests unitaires
cargo test

# Tests d'intégration (requiert un fichier .gpkg)
cargo test --test integration -- --ignored
```

## Limitations

- Dimensions maximales : 20 000 × 20 000 pixels
- Seuls les types POLYGON et MULTIPOLYGON sont supportés
- Les géométries qui échouent à la reprojection sont ignorées silencieusement

## Licence

MIT

## Auteur

Généré avec [Claude Code](https://claude.ai/code)

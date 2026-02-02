# Support des géométries GeoJSON avec `"type":""`

**Date**: 2026-02-02
**Statut**: Approuvé pour implémentation

## Contexte

Certains outils génèrent des fichiers GeoJSON malformés où le champ `type` d'une géométrie est une chaîne vide (`"type":""`). Ces fichiers contiennent des coordonnées valides au format MultiPolygon, mais le parser `geojson` échoue car le type vide n'est pas reconnu.

## Objectif

Permettre au lecteur GeoJSON d'accepter ces fichiers en interprétant automatiquement `"type":""` comme `"type":"MultiPolygon"`.

## Approche technique

### Solution choisie: Pré-processing avec remplacement de chaîne

Extension du pattern existant (ligne 32 de `geojson.rs`) qui corrige déjà les double-quotes CSV.

**Emplacement**: `src/geojson.rs`, fonction `GeojsonReader::open()`

**Logique**:
```rust
// Fix empty type field ("type":"" -> "type":"MultiPolygon")
// Handles malformed GeoJSON where type field is empty
let content = content.replace(r#""type":"""#, r#""type":"MultiPolygon""#);
```

**Positionnement**: Juste après le fix des double-quotes, avant le parsing.

### Alternatives considérées

**Option B - Parsing manuel avec serde_json**: Plus précis mais significativement plus complexe. Rejeté car le remplacement simple suffit.

**Option C - Regex contextuel**: Plus robuste contre les faux positifs, mais le pattern `"type":""` n'apparaît jamais dans un GeoJSON valide, donc inutile.

## Cas d'usage supportés

### 1. Géométrie racine avec type vide
```json
{
  "type": "",
  "coordinates": [[[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]]]
}
```

### 2. Feature avec géométrie type vide
```json
{
  "type": "Feature",
  "geometry": {
    "type": "",
    "coordinates": [[[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]]]
  }
}
```

### 3. FeatureCollection avec mix
```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": {
        "type": "",
        "coordinates": [[[[...]]]
      }
    },
    {
      "type": "Feature",
      "geometry": {
        "type": "Polygon",
        "coordinates": [[[...]]]
      }
    }
  ]
}
```

## Tests requis

### Tests unitaires (dans `src/geojson.rs`)

1. **`test_parse_empty_type_root_geometry`**
   - Input: Géométrie racine avec `"type":""`
   - Expected: Une géométrie extraite

2. **`test_parse_empty_type_in_feature`**
   - Input: Feature avec geometry.type vide
   - Expected: Géométrie correctement extraite

3. **`test_parse_empty_type_in_featurecollection`**
   - Input: FeatureCollection avec plusieurs features, certaines avec type vide
   - Expected: Toutes les géométries extraites

## Edge cases et limitations

### Edge cases gérés
- **Espaces dans le pattern**: `"type": ""` (avec espace) ne sera pas matché. Acceptable car les fichiers problématiques observés n'ont pas d'espaces.
- **Faux positifs**: Théoriquement possible dans des propriétés, mais `"type":""` n'est pas un pattern réaliste dans les propriétés GeoJSON.

### Limitations connues
- **Assumption MultiPolygon**: Toutes les géométries avec type vide sont traitées comme MultiPolygons. Si les coordonnées sont incompatibles, le parser `geojson` échouera (comportement acceptable).
- **Pas de validation préalable**: Aucune vérification que les coordonnées correspondent bien à un MultiPolygon avant le remplacement.

## Impact

### Compatibilité
- **Aucune régression**: Les fichiers GeoJSON valides ne contiennent jamais `"type":""`
- **Backward compatible**: Tous les tests existants restent valides

### Performance
- **Négligeable**: Un simple `.replace()` sur la string avant parsing
- **Ordre**: O(n) où n = taille du fichier (déjà chargé en mémoire)

### Maintenance
- **Suit le pattern établi**: Cohérent avec le fix des double-quotes (ligne 32)
- **Code simple**: Facile à comprendre et maintenir

## Documentation à mettre à jour

- **Code**: Commentaires clairs expliquant le fix
- **CLAUDE.md**: Ajouter dans "Lessons Learned" comme cas de malformed data handling
- **Tests**: Tests unitaires documentent le comportement attendu

## Implémentation

### Fichiers modifiés
- `src/geojson.rs`: Ajout du fix et des tests

### Ordre d'implémentation
1. Ajouter les tests unitaires (TDD)
2. Implémenter le fix
3. Vérifier que tous les tests passent
4. Mettre à jour la documentation

## Validation

- [ ] Tests unitaires passent
- [ ] Tests d'intégration existants passent
- [ ] Testé manuellement avec fichier GeoJSON malformé réel
- [ ] Documentation mise à jour

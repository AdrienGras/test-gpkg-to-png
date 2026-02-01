# Design Document : Moteur de Rendu Scanline Haute Performance

**Date** : 2026-02-01
**Statut** : Validé
**Auteur** : Claude

## 1. Objectif
Remplacer l'algorithme actuel de type Ray-Casting (test par pixel) par un algorithme de Scanline Fill (remplissage par balayage) parallélisé par bandes horizontales. Cette approche vise à éliminer la contention du Mutex et à réduire la complexité algorithmique pour les polygones complexes.

## 2. Architecture Technique

### 2.1. Structure des données
- **`Edge`** : Représente un segment de ligne non horizontal.
    - `y_max` : La coordonnée Y maximale du segment.
    - `x_current` : La coordonnée X actuelle à l'intersection de la ligne de balayage.
    - `dx_per_scanline` : La pente inverse (1/m).
- **`EdgeTable`** : Un dictionnaire/vecteur indexé par Y contenant les listes d'arêtes qui commencent à cette ligne.
- **`ActiveEdgeTable` (AET)** : La liste des arêtes intersectant la ligne de balayage actuelle, triée par `x_current`.

### 2.2. Parallélisation par Bandes (Bands)
L'image est divisée en $N$ bandes horizontales indépendantes (ex: 64 pixels de haut).
1. **Distribution** : Utilisation de `rayon` pour traiter chaque bande en parallèle via `image.chunks_mut()`.
2. **Indépendance** : Chaque thread travaille sur un slice de mémoire exclusif, supprimant le besoin de `Mutex` sur les pixels.
3. **Optimisation Spatiale** : Seules les arêtes intersectant la zone Y de la bande sont prises en compte par chaque thread.

## 3. Flux de Données (Pipeline)

1. **Phase de Préparation (Séquentielle)** :
    - Conversion des polygones en segments de lignes (bords).
    - Projection des coordonnées monde vers écran.
    - Filtrage des segments horizontaux.
    - Construction d'une Global Edge Table (GET).

2. **Phase de Rendu (Parallèle par Bande)** :
    - Pour chaque bande de l'image :
        - Initialiser une AET locale.
        - Pour chaque ligne Y de la bande :
            - Ajouter les nouvelles arêtes de la GET commençant à Y.
            - Retirer les arêtes dont `y_max` est atteint.
            - Trier l'AET par `x_current`.
            - Remplir les pixels entre les paires d'intersections (Règle Parité).
            - Mettre à jour `x_current` pour chaque arête (+$dx$).

3. **Phase de Contour (Stroke)** :
    - Parallélisation similaire sur les segments du contour pour éviter les conflits d'écriture.

## 4. Gains de Performance Atteints
- **Algorithmique** : Passage de $O(Points \times Pixels)$ à $O(Points \log Points + Pixels\_Remplis)$.
- **Concurrency** : Zéro contention de verrouillage (lock-free pixel writing).
- **Load Balancing** : Meilleure répartition des gros polygones sur tous les cœurs.

## 5. Précision et Cas Limites
- **Règle de Parité** : Utilisation stricte de la règle "even-odd" pour gérer les trous (holes).
- **Sub-pixel Accuracy** : Utilisation de calculs en flottants pour `x_current` et `dx` afin d'éviter les dérives visuelles.
- **Clamping** : Gestion robuste des bords de l'image.

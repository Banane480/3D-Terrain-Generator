# 🌍 3D Terrain Generator — Rust Vulkan Engine

Un moteur de rendu 3D haute définition et générateur de monde ouvert procédural en vue première personne, entièrement conçu en **Rust** avec l'API **Vulkan** (`ash`).

Ce projet propose un **terrain continu haute définition**, sans cubes ni voxels, doté de plusieurs biomes réalistes, de textures PBR triplanaires, d'un éclairage atmosphérique et d'une physique de marcheur FPS avec bascule en caméra libre.

---

## ✨ Fonctionnalités Clés

* **Moteur Vulkan 1.2+ pur Rust** :
  * Bindings bas niveau `ash` pour un contrôle total du pipeline graphique.
  * Gestion mémoire VRAM haute performance avec `gpu-allocator`.
  * Double-buffering avec 2 trames en vol (*Frames in Flight*), sémaphores et fences.
  * Tampon de profondeur matériel `D32_SFLOAT`.
  * Shaders GLSL compilés en SPIR-V à la volée via script de build `naga` (aucune dépendance C++ ou SDK externe requise).

* **Génération Procédurale HD (Zéro Voxel)** :
  * **Relief fractal FBM** combinant bruit Simplex et Perlin.
  * **Domain Warping** : Déformation vectorielle créant des chaînes de montagnes naturelles et des vallées sinueuses.
  * **Normales par différences centrales** : Éclairage fluide sans discontinuités entre maillages.
  * **Répartition multi-biomes** : Plages de sable fin, plaines verdoyantes, forêts denses, falaises rocheuses alpines et sommets enneigés.

* **Rendu PBR & Textures Réelles** :
  * Intégration de 4 ensembles de textures PBR 1K (CC0 ambientCG) : Sable (`Ground054`), Herbe (`Ground037`), Roche (`Rock020`), Neige (`Snow006`).
  * **Texture 2D Array Vulkan** (`sampler2DArray`) avec filtrage linéaire et **anisotropie 16x**.
  * **Triplanar Mapping** : Projection triplanaire sur la roche alpine pour éliminer tout étirement vertical sur les parois abruptes.
  * **Éclairage atmosphérique** : Soleil directionnel, illumination hémisphérique du ciel, reflets spéculaires et brouillard exponentiel de distance.

* **Gameplay & Caméra Première Personne** :
  * **Mode Marcheur FPS** : Adhérence au relief, détection d'impact, gravité et saut réaliste.
  * **Mode Flycam (Vol libre 6-DOF)** : Déplacement omnidirectionnel avec la touche `F`.
  * **Inversion de l'axe vertical de la souris** : Touche `I` pour basculer en temps réel entre visée normale et inversée.
  * **Support natif ZQSD (AZERTY) et WASD (QWERTY)**.

---

## 🎮 Contrôles en Jeu

| Touche | Action |
|:---:|---|
| **Z / W** | Avancer |
| **S** | Reculer |
| **Q / A** | Pas chassé à gauche |
| **D** | Pas chassé à droite |
| **Souris** | Orientation de la vue (360°) |
| **Espace** | Sauter (Marcheur) / Monter (Flycam) |
| **Maj (Shift)** | Courir (Sprint accéléré) |
| **Ctrl / C** | S'accroupir (Marcheur) / Descendre (Flycam) |
| **F** | **Basculer entre Mode Marcheur FPS $\leftrightarrow$ Flycam libre** |
| **I** | **Basculer l'axe vertical de la souris (Normal $\leftrightarrow$ Inversé)** |
| **Échap** | Libérer ou verrouiller le curseur de la souris |

---

## 🚀 Compilation et Lancement

### Prérequis
* [Rust](https://rustup.rs/) (Édition 2021, rustc 1.70 ou plus récent).
* Carte graphique compatible **Vulkan 1.2+** avec pilotes à jour (NVIDIA, AMD ou Intel).

### Lancement direct
Clonez le dépôt et lancez le projet en mode optimisé :

```bash
git clone https://github.com/Banane480/3D-Terrain-Generator.git
cd 3D-Terrain-Generator
cargo run --release
```

---

## 🏗️ Architecture du Projet

```text
├── Cargo.toml                  # Configuration des dépendances (ash, gpu-allocator, winit, glam, noise...)
├── build.rs                    # Compilateur GLSL vers SPIR-V via Naga
├── shaders/
│   ├── terrain.vert            # Vertex shader GLSL (projections, positions monde, poids biomes)
│   └── terrain.frag            # Fragment shader GLSL (PBR, triplanar mapping, brouillard)
├── assets/
│   └── textures/               # Textures PBR (sable, herbe, roche, neige)
└── src/
    ├── main.rs                 # Initialisation Winit 0.30, boucle de rendu, gestion FPS
    ├── camera.rs               # Caméra première personne (physique FPS, vol libre, uniformes)
    ├── input.rs                # Machine à états des entrées (clavier, souris, verrous)
    ├── terrain/
    │   ├── mod.rs              # Exports du module terrain
    │   ├── generator.rs        # Algorithmes de bruit fractal, domain warping, biomes
    │   ├── mesh.rs             # Générateur de maillage continu haute définition (TerrainVertex)
    │   └── world.rs            # Agrégation et streaming des chunks de terrain
    └── vulkan/
        ├── mod.rs              # Exports du backend Vulkan
        ├── context.rs          # Instance, Surface, sélection GPU dédié, Logical Device
        ├── swapchain.rs        # SwapchainKHR, D32 Depth Buffer, RenderPass, Framebuffers
        ├── allocator.rs        # Gestionnaire VRAM (gpu-allocator)
        ├── buffer.rs           # Buffers GPU (Uniform mappé et Vertex/Index en VRAM via Staging)
        ├── texture.rs          # Texture 2D Array embarquée avec Sampler anisotrope
        ├── pipeline.rs         # Graphics Pipeline, Descriptor Sets, dynamic viewports
        └── sync.rs             # Sémaphores et Fences (Frames in Flight)
```

---

## 🗺️ Roadmap & Évolutions Futures

- [x] Rendu de terrain continu procédural HD sans voxel.
- [x] Biomes multiples avec pondération continue (altitude, pente, humidité).
- [x] Textures PBR avec projection triplanaire sur falaises.
- [x] Caméra première personne hybride (Marcheur FPS + Flycam).
- [ ] **Cascaded Shadow Maps (CSM)** : Ombres portées dynamiques du soleil avec filtrage PCF.
- [ ] **Plan d'eau dynamique** : Surface d'eau à $y=0$ avec ondes de Gerstner, réfraction et Fresnel.
- [ ] **Système CDLOD / Quadtree** : Niveaux de détail adaptatifs avec geomorphing skirts pour terrain infini.
- [ ] **Végétation instanciée** : Arbres et herbe distribués par densité de biome et rendus via GPU Instancing.

---

## 📄 Licence

Ce projet est sous licence [MIT](LICENSE). Les textures proviennent de [ambientCG](https://ambientcg.com/) sous licence libre [Creative Commons CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/).

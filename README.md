# 🌍 3D Terrain Generator — Rust Vulkan Engine

Un moteur de rendu 3D haute définition et générateur de monde ouvert procédural en vue première personne, entièrement conçu en **Rust** avec l'API **Vulkan** (`ash`).

Ce projet propose un **terrain continu haute définition**, sans cubes ni voxels, doté de plusieurs biomes réalistes, de textures PBR triplanaires, d'un dôme atmosphérique, d'un océan animé, d'une riche **végétation 3D instanciée** animée par le vent, et d'une physique de marcheur FPS avec bascule en caméra libre.

---

## ✨ Fonctionnalités Graphiques & Techniques

* **Moteur Vulkan 1.2+ pur Rust ultra-optimisé** :
  * Bindings bas niveau `ash` pour un contrôle absolu du pipeline graphique.
  * Gestion mémoire VRAM haute performance avec `gpu-allocator` (Device-Local VRAM & Staging Buffers).
  * Double-buffering avec 2 trames en vol (*Frames in Flight*), sémaphores et fences.
  * Tampon de profondeur matériel `D32_SFLOAT`.
  * Rendu multi-passes optimisé (Ciel $\rightarrow$ Terrain $\rightarrow$ Végétation $\rightarrow$ Océan) en seulement **5 draw calls par trame** pour des performances maximales (+100 FPS).
  * 8 shaders GLSL compilés en SPIR-V à la volée via script de build `naga` (zéro dépendance C++ externe).

* **🌲 Végétation 3D Instanciée & Dynamique (Instanced Rendering)** :
  * **Modèles 3D procéduraux** : Sapins de montagne (conifères alpins), Arbres feuillus des plaines (chênes volumétriques) et Buissons de sous-bois.
  * **Implantation biologique procédurale** : Répartition organique selon l'altitude, la pente, l'humidité et le biome (des centaines d'arbres et buissons répartis organiquement).
  * **Animation dynamique du vent** : Shaders de sommet déformant la cime et les feuilles avec turbulence sinusoïdale en temps réel (`wind sway`).
  * **Éclairage translucide & Subsurface Scattering (SSS)** : Pénétration de la lumière à travers le feuillage et occlusion ambiante volumétrique.
  * **Instanced Rendering Vulkan** : Rendu de près de 1 000 arbres et buissons en seulement **3 draw calls instanciés** avec zéro overhead CPU.

* **🌅 Atmosphère & Ciel Dynamique (Rayleigh & Mie Scattering)** :
  * Dôme de ciel procédural avec dégradé d'élévation atmosphérique.
  * Disque solaire éclatant avec diffusion de Mie, corona lumineuse et halo doré sur l'horizon.
  * Brume d'horizon douce fusionnant harmonieusement les reliefs lointains.

* **🌊 Océan & Surface d'Eau Animée** :
  * Nappe maillée au niveau de la mer animée par vagues sinusoïdales multi-fréquences.
  * **Reflets de Fresnel** : L'eau reflète le ciel et le soleil sous les angles rasants.
  * Gradient de profondeur marine : Vert turquoise tropical dans les hauts-fonds $\rightarrow$ Bleu saphir profond en haute mer.
  * Écume dynamique sur la crête des vagues et reflets spéculaires solaires intenses.

* **🎨 Rendu PBR & Post-Processing Cinématographique** :
  * 4 ensembles de textures PBR 1K (CC0 ambientCG) : Sable (`Ground054`), Herbe (`Ground037`), Roche (`Rock020`), Neige (`Snow006`).
  * **Texture 2D Array Vulkan** (`sampler2DArray`) avec filtrage linéaire et **anisotropie 16x**.
  * **Triplanar Mapping** sur les falaises et parois abruptes.
  * **ACES Filmic Tone Mapping** intégré sur tous les shaders pour un rendu visuel cinématographique éclatant et riche en contrastes.

* **🎮 Contrôles FPS & Exploration** :
  * **Mode Marcheur FPS** : Détection d'impact avec le relief continu, gravité, amortissement et saut.
  * **Mode Flycam (Vol libre 6-DOF)** : Touche `F` pour survoler librement les paysages.
  * **Inversion Axe Souris** : Touche `I` pour basculer en temps réel entre visée normale et inversée.
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
```bash
git clone https://github.com/Banane480/3D-Terrain-Generator.git
cd 3D-Terrain-Generator
cargo run --release
```

Ou exécuter directement le binaire autonome :
```powershell
.\vulkan_terrain_engine.exe
```

---

## 🏗️ Architecture du Projet

```text
├── Cargo.toml                  # Dépendances (ash, gpu-allocator, winit, glam, noise...)
├── build.rs                    # Compilateur GLSL vers SPIR-V via Naga (8 shaders)
├── shaders/
│   ├── terrain.vert            # Vertex shader du terrain continu
│   ├── terrain.frag            # Fragment shader terrain (PBR, Triplanar, ACES)
│   ├── vegetation.vert         # Vertex shader instancié avec animation du vent
│   ├── vegetation.frag         # Fragment shader feuillage (Subsurface scattering, AO, ACES)
│   ├── sky.vert                # Vertex shader du dôme atmosphérique
│   ├── sky.frag                # Fragment shader ciel (Rayleigh, Mie, disque solaire)
│   ├── water.vert              # Vertex shader vagues animées
│   └── water.frag              # Fragment shader océan (Fresnel, écume, spéculaire)
├── assets/
│   └── textures/               # Textures PBR (sable, herbe, roche, neige)
└── src/
    ├── main.rs                 # Initialisation Winit 0.30, boucle de rendu, passes Vulkan
    ├── camera.rs               # Caméra première personne (physique FPS, vol libre, uniformes)
    ├── input.rs                # Machine à états des entrées (clavier, souris, verrous)
    ├── environment/
    │   ├── mod.rs              # Exports environnement
    │   ├── sky.rs              # Générateur géométrique de dôme céleste
    │   └── water.rs            # Générateur de plan d'eau marin
    ├── vegetation/
    │   ├── mod.rs              # Exports végétation
    │   ├── mesh.rs             # Modèles 3D procéduraux (Sapins, Chênes, Buissons)
    │   ├── instance.rs         # Attributs d'instance Vulkan
    │   └── spawner.rs          # Algorithme de distribution organique selon biomes et pente
    ├── terrain/
    │   ├── mod.rs              # Exports du module terrain
    │   ├── generator.rs        # Relief fractal FBM, domain warping, biomes
    │   ├── mesh.rs             # Générateur de maillage continu haute définition
    │   └── world.rs            # Agrégation et streaming des chunks
    └── vulkan/
        ├── mod.rs              # Exports du backend Vulkan
        ├── context.rs          # Instance, Surface, sélection GPU, Logical Device
        ├── swapchain.rs        # SwapchainKHR, D32 Depth Buffer, RenderPass, Framebuffers
        ├── allocator.rs        # Gestionnaire VRAM (gpu-allocator)
        ├── buffer.rs           # Buffers GPU (Uniforms et Vertex/Index en VRAM via Staging)
        ├── texture.rs          # Texture 2D Array PBR 1K avec Sampler anisotrope
        ├── pipeline.rs         # Pipelines graphiques (Terrain, Végétation, Ciel, Eau)
        └── sync.rs             # Sémaphores et Fences (Frames in Flight)
```

---

## 📄 Licence

Ce projet est sous licence [MIT](LICENSE).

# 18: Distribuer le binaire sur les trois systèmes hôtes

**What to build:** Produire des binaires installables sur Linux, macOS et Windows avec un parcours documenté d’installation et de première utilisation.

**Blocked by:** 01 — Identifier et diagnostiquer un appareil.

**Status:** in-progress

**Blocker:** Native macOS ARM64 and Windows MSVC runtime/CI artifact checks require runners unavailable locally. Software pipeline and Linux validation integrated; no remote publication authorized.

## Acceptance criteria

- [ ] Les systèmes et architectures hôtes pris en charge sont explicitement définis ; les artefacts correspondants sont générés de façon reproductible par un processus documenté.
- [ ] Installation, lancement, aide, version et diagnostic HTTP fonctionnent sans environnement Rust chez l’utilisateur.
- [ ] Les contrôles automatisables vérifient l’aide et le parcours HTTP contre une cible de test sur chaque système hôte annoncé.
- [ ] Les conventions de configuration personnelle et chemins sont compatibles avec les systèmes visés ; les artefacts restent autonomes selon les prérequis annoncés.
- [ ] Le support d’exécution du binaire AWTRIX headless est annoncé séparément du support du client CLI ; sa disponibilité n’est pas supposée identique sur les trois systèmes.
- [ ] La génération des artefacts est testée sans publier automatiquement de release ou pousser vers un remote non demandé.

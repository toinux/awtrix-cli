# 18: Distribuer le binaire sur les trois systèmes hôtes

**What to build:** Produire des binaires installables sur Linux, macOS et Windows avec un parcours documenté d’installation et de première utilisation.

**Blocked by:** 01 — Identifier et diagnostiquer un appareil.

**Status:** in-progress

**Blocker:** Native CI now available and branch push authorized. Run 37658583268 passes Linux but fails macOS ARM64 and Windows MSVC Clippy on non-Linux headless code/test gating; builds/HTTP checks on those hosts and all artifact jobs are therefore not completed. Isolated correction worktree prepared, but Luna implementer dispatch failed twice with an OpenCode session database insertion error. See automation.md for resume evidence.

## Acceptance criteria

- [ ] Les systèmes et architectures hôtes pris en charge sont explicitement définis ; les artefacts correspondants sont générés de façon reproductible par un processus documenté.
- [ ] Installation, lancement, aide, version et diagnostic HTTP fonctionnent sans environnement Rust chez l’utilisateur.
- [ ] Les contrôles automatisables vérifient l’aide et le parcours HTTP contre une cible de test sur chaque système hôte annoncé.
- [ ] Les conventions de configuration personnelle et chemins sont compatibles avec les systèmes visés ; les artefacts restent autonomes selon les prérequis annoncés.
- [ ] Le support d’exécution du binaire AWTRIX headless est annoncé séparément du support du client CLI ; sa disponibilité n’est pas supposée identique sur les trois systèmes.
- [ ] La génération des artefacts est testée sans publier automatiquement de release ou pousser vers un remote non demandé.

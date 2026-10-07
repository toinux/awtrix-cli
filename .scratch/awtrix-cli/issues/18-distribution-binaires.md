# 18: Distribuer le binaire sur les trois systèmes hôtes

**What to build:** Produire des binaires installables sur Linux, macOS et Windows avec un parcours documenté d’installation et de première utilisation.

**Blocked by:** 01 — Identifier et diagnostiquer un appareil.

**Status:** done

**Evidence:** Luna correction 72f75b5 independently reviewed without blocking findings, integrated as c48dfe6. Native CI [37660685612](https://github.com/toinux/awtrix-cli/actions/runs/37660685612) passes all three host jobs (format, Clippy, release build, release-binary help/version/HTTP) and all three artifact jobs. Linux full suite and explicit real-headless integration pass. See automation.md for artifact identifiers and acceptance evidence. No release published.

## Acceptance criteria

- [x] Les systèmes et architectures hôtes pris en charge sont explicitement définis ; les artefacts correspondants sont générés de façon reproductible par un processus documenté.
- [x] Installation, lancement, aide, version et diagnostic HTTP fonctionnent sans environnement Rust chez l’utilisateur.
- [x] Les contrôles automatisables vérifient l’aide et le parcours HTTP contre une cible de test sur chaque système hôte annoncé.
- [x] Les conventions de configuration personnelle et chemins sont compatibles avec les systèmes visés ; les artefacts restent autonomes selon les prérequis annoncés.
- [x] Le support d’exécution du binaire AWTRIX headless est annoncé séparément du support du client CLI ; sa disponibilité n’est pas supposée identique sur les trois systèmes.
- [x] La génération des artefacts est testée sans publier automatiquement de release ou pousser vers un remote non demandé.

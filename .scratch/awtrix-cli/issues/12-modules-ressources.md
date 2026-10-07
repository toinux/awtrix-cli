# 12: Gérer les modules Berry et les ressources

**What to build:** Déployer et gérer les modules Berry et fichiers nécessaires aux scripts, notamment les icônes, sans confondre installation de code et upload générique.

**Blocked by:** 01 — Identifier et diagnostiquer un appareil.

**Status:** done

## Acceptance criteria

- [ ] Les modules Berry peuvent être gérés selon leurs routes officielles et ne sont pas présentés comme applications de la rotation.
- [ ] Les opérations utiles de liste, lecture/téléchargement, upload et suppression de fichiers sont exposées selon l’API disponible.
- [ ] Les types d’icônes GIF/JPEG, leur identification sensible à la casse et les règles de payload inline sont respectés selon la version ; renommer une extension ne valide pas un format incompatible.
- [ ] Les tailles, destinations et capacités sont validées en fonction de la route/variante ; les erreurs locales et distantes sont distinguées.
- [ ] Un remplacement ou renommage n’est pas présenté comme une réécriture automatique des références des applications.
- [ ] Aide, descriptions structurées et tests CLI couvrent source de module, multipart/fichiers, format invalide et limites pertinentes.

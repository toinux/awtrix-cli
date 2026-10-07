# 14: Suivre les éléments du projet et les retirer explicitement

**What to build:** Suivre les éléments réellement déployés par projet et cible pour retirer explicitement les anciens éléments sans toucher aux contenus étrangers.

**Blocked by:** 13 — Initialiser et déployer un projet TOML.

**Status:** in-progress

## Acceptance criteria

- [ ] Le mécanisme de suivi est défini et documenté ; il distingue identité du projet et cible effective et enregistre les opérations réellement réussies.
- [ ] Un élément retiré du manifeste n’est pas supprimé lors d’un déploiement normal.
- [ ] Une opération explicite permet de retirer les éléments précédemment suivis devenus obsolètes, avec rapport des suppressions.
- [ ] Les éléments sans provenance de suivi pour ce projet/cette cible restent intacts. Un suivi absent ou invalide ne déclenche pas de suppression large.
- [ ] Après échec partiel, le suivi et les rapports restent cohérents avec les effets réellement connus ; les effets incertains sont signalés.
- [ ] Aide, descriptions et tests CLI couvrent deux projets, deux cibles, changement de manifeste, contenu étranger et interruption partielle.

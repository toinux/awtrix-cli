# 03: Afficher une progression avec une application poussée

**What to build:** Créer, mettre à jour et supprimer une application poussée pour présenter la progression d’un travail sur l’écran.

**Blocked by:** 01 — Identifier et diagnostiquer un appareil.

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Un agent peut envoyer un contenu nommé, mettre à jour ce même contenu et le supprimer avec des commandes composables.
- [ ] Les paramètres d’affichage et d’expiration pris en charge sont décrits ; les entrées invalides sont détectées plutôt que silencieusement remplacées par des valeurs par défaut.
- [ ] Les limites pertinentes de la route et de la variante sont respectées ; une capacité indisponible est signalée explicitement.
- [ ] Le résultat indique l’opération acceptée sans prétendre que l’application est immédiatement visible. La documentation rappelle la perte des applications poussées au redémarrage.
- [ ] Les commandes ont aide, descriptions et schémas structurés, sorties humaines et JSON compact.
- [ ] Les tests CLI vérifient les payloads, création/remplacement/suppression, erreurs de validation, limites et erreurs distantes.

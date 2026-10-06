# 11: Vérifier le fonctionnement d’un script

**What to build:** Observer un script pendant une fenêtre configurable et réunir état, erreurs, logs et capture optionnelle en un diagnostic unique.

**Blocked by:** 07 — Lire et déployer un script avec protection contre les conflits ; 09 — Consulter et suivre les logs ; 10 — Capturer l’écran dans un fichier.

**Status:** ready-for-agent

## Acceptance criteria

- [ ] La commande peut vérifier un script existant et proposer un parcours déploiement puis vérification, sans écraser silencieusement la protection du déploiement.
- [ ] La durée est bornée ; le rapport sépare source enregistré, démarrage vérifié, période observée et données non disponibles.
- [ ] Les erreurs immédiates et runtime ultérieures sont détectées ; scripts désactivés ou absents ne sont pas présentés comme exécutés correctement.
- [ ] Les logs sont collectés par curseur pendant la fenêtre et une capture peut être ajoutée sous forme d’artefact.
- [ ] L’absence d’erreur observée n’est pas décrite comme preuve générale de correction ; l’historique de logs limité et l’échec d’une collecte sont signalés.
- [ ] Aide, descriptions et tests CLI couvrent succès observé, erreur retardée, script non exécuté, délai et diagnostic incomplet.

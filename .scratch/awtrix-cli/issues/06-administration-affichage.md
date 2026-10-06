# 06: Administrer l’affichage et redémarrer

**What to build:** Lire et modifier les réglages d’affichage, piloter luminosité et alimentation, consulter la configuration système et demander un redémarrage.

**Blocked by:** 01 — Identifier et diagnostiquer un appareil.

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Les opérations de luminosité, alimentation et réglages d’affichage suivent les routes et capacités de la variante.
- [ ] Les types et valeurs sont validés ; un PATCH invalide n’est pas présenté comme partiellement appliqué lorsque le contrat le rejette globalement.
- [ ] La configuration système peut être lue sans ajout de commandes d’écriture réseau/MQTT ni exposition de secrets.
- [ ] Le redémarrage distingue une demande acceptée d’un retour en ligne effectivement observé ; une rupture de connexion ambiguë n’est pas déclarée succès certain.
- [ ] Les erreurs de sauvegarde sont rapportées avec les informations disponibles sur application et persistance, sans masquer une éventuelle modification déjà réalisée.
- [ ] Aide, descriptions structurées et tests CLI couvrent succès, valeurs invalides, incompatibilité, rejet global et redémarrage.

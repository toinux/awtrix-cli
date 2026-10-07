# 08: Gérer le cycle de vie et la configuration des scripts

**What to build:** Activer, désactiver, supprimer et configurer un script, avec accès aux diagnostics Berry utiles à sa correction.

**Blocked by:** 07 — Lire et déployer un script avec protection contre les conflits.

**Status:** in-progress

## Acceptance criteria

- [ ] Les commandes de lecture d’état, activation, désactivation et suppression utilisent les contrats corrects, notamment les formes de payload attendues.
- [ ] La configuration et les données de script pertinentes sont consultables/modifiables selon les capacités officielles ; les effets de rechargement et persistance sont expliqués.
- [ ] Les erreurs de script exposent message, ligne et hook lorsqu’ils existent, sans inventer les informations absentes.
- [ ] Le résultat décrit l’état observé et n’assimile pas une commande acceptée à l’absence future d’erreur runtime.
- [ ] Aide, descriptions structurées, JSON compact et sélection de champs sont disponibles.
- [ ] Les tests CLI vérifient cycle de vie, configuration, erreurs partielles de diagnostic et réponses invalides ou incompatibles.

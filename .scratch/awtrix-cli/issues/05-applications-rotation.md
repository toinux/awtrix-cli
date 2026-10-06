# 05: Piloter les applications et la rotation

**What to build:** Consulter les applications et leur participation à la rotation, sélectionner une application et gérer la rotation selon les possibilités de l’appareil.

**Blocked by:** 01 — Identifier et diagnostiquer un appareil.

**Status:** ready-for-agent

## Acceptance criteria

- [ ] La liste expose les informations utiles disponibles, dont origine, activation, présence, participation à la rotation et erreurs, avec détails sur demande.
- [ ] Une application peut être sélectionnée et la rotation consultée/modifiée selon le contrat officiel vérifié.
- [ ] La sélection ne prétend pas supprimer une notification ni garantir que l’application est actuellement visible.
- [ ] Une application absente, un paramètre invalide ou une opération incompatible produit une erreur explicite.
- [ ] Aide, descriptions, schémas et sélection de champs couvrent les commandes.
- [ ] Les tests CLI vérifient lectures et mutations, préservation des valeurs non visées et erreurs pertinentes pour les variantes.

# 04: Envoyer et retirer des notifications

**What to build:** Envoyer une notification, gérer ses options de file et la retirer, en donnant à l’agent un résultat honnête sur son acceptation.

**Blocked by:** 01 — Identifier et diagnostiquer un appareil.

**Status:** in-progress

## Acceptance criteria

- [ ] Les notifications peuvent être envoyées avec les options documentées de maintien, file et réveil, et retirées selon les possibilités de l’API.
- [ ] Les paramètres sont typés et validés ; les noms et options ne sont pas présentés comme des garanties de déduplication non fournies par AWTRIX.
- [ ] Une notification acceptée n’est pas déclarée nécessairement visible ; la sélection dans la rotation ne sert pas de preuve de visibilité.
- [ ] Aucun retry automatique aveugle ne duplique un POST après une réponse réseau incertaine ; le résultat expose cette incertitude.
- [ ] L’aide et la description structurée expliquent les options et résultats, avec sorties humaines/JSON et erreurs stables.
- [ ] Les tests CLI couvrent payloads, suppression, refus distant, file pleine selon le contrat applicable et réponse perdue sans second envoi.

# 15: Lancer et arrêter une cible headless

**What to build:** Piloter le cycle de vie d’une instance AWTRIX Linux headless à partir d’un binaire fourni ou configuré, pour le développement interactif et les futurs tests isolés.

**Blocked by:** 01 — Identifier et diagnostiquer un appareil.

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Une instance peut être lancée avec paramètres documentés de port, dimensions et données ; le CLI attend sa disponibilité HTTP pendant un délai borné et retourne une cible utilisable.
- [ ] Le mode isolé crée ses propres données ; le mode persistant permet leur réutilisation pour le développement interactif.
- [ ] L’arrêt et le nettoyage ne concernent que les instances créées et possédées par le CLI ; une instance externe réutilisée n’est pas arrêtée implicitement.
- [ ] Un démarrage échoué, délai dépassé ou interruption libère les ressources créées et rapporte les informations utiles.
- [ ] Le binaire est fourni/configuré, sans téléchargement automatique ; un environnement incompatible ou binaire absent produit une erreur explicite.
- [ ] L’aide et la description indiquent les limites headless : pas de validation des capteurs, audio ni budgets réels ESP32.
- [ ] Les tests du cycle de vie couvrent succès/échec et propriété des processus ; un véritable AWTRIX headless est joint via HTTP pour valider le parcours, avec environnement requis documenté.

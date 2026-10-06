# 07: Lire et déployer un script avec protection contre les conflits

**What to build:** Lire un source Berry distant et déployer un fichier isolé en protégeant les mises à jour contre les modifications concurrentes lorsque AWTRIX le permet.

**Blocked by:** 01 — Identifier et diagnostiquer un appareil.

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Le source peut être récupéré exactement et un fichier Berry brut déployé sans manifeste obligatoire ; les noms et contraintes de la route sont validés.
- [ ] Le parcours conserve ou accepte un source distant de référence et utilise la mise à jour conditionnelle disponible. Une nouvelle lecture juste avant l’écriture ne remplace pas silencieusement la référence initiale.
- [ ] Un conflit est exposé sans écraser le source distant. Le remplacement inconditionnel est demandé explicitement ; le parcours de création d’un script absent est défini.
- [ ] Si la protection manque, le CLI le signale et décrit la garantie réelle, sans simuler une atomicité par lecture puis écriture.
- [ ] Un HTTP réussi avec erreur Berry est un échec opérationnel. Le résultat distingue source enregistré, démarrage vérifié et état inconnu ; des scripts globalement désactivés ne sont pas déclarés compilés/démarrés.
- [ ] Les garanties de restauration sont celles de la route utilisée et ne couvrent pas les erreurs runtime ultérieures. Le plafond JSON n’est pas automatiquement appliqué au source brut.
- [ ] Aide, schémas et tests CLI couvrent création, modification, conflit, remplacement explicite, capacité absente, erreur de compilation/setup et source simplement sauvegardé.

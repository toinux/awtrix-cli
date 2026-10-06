# 02: Gérer les profils et sélectionner la cible

**What to build:** Enregistrer des profils d’appareil nommés, choisir un profil par défaut et expliquer la cible effective de chaque commande.

**Blocked by:** 01 — Identifier et diagnostiquer un appareil.

**Status:** in-progress

## Acceptance criteria

- [ ] L’utilisateur peut créer, consulter, modifier et supprimer des profils et définir le profil par défaut.
- [ ] La sélection suit la priorité commande explicite, environnement, référence de projet lorsqu’elle sera disponible, puis profil global par défaut. Les noms des options et variables sont documentés.
- [ ] Le diagnostic expose la cible effective et l’origine de sa sélection ; un profil absent ou une configuration invalide échoue explicitement.
- [ ] Les identifiants sont fournis par la configuration personnelle ou l’environnement. Les sorties et descriptions ne révèlent pas les secrets.
- [ ] La commande explicite et les variables d’environnement permettent un usage sans profil enregistré.
- [ ] Aide, descriptions structurées, résultats JSON compacts et sélection de champs couvrent les nouvelles commandes.
- [ ] Les tests CLI utilisent une configuration temporaire isolée et vérifient persistance, sélection, authentification et absence de fuite de secrets. Le raccordement au manifeste sera testé dans le ticket 13.

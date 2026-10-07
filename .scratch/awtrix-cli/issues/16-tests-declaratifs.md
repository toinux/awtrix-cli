# 16: Exécuter les tests déclaratifs d’un projet

**What to build:** Déployer et tester un projet avec assertions sur scripts, état et logs dans une instance headless isolée ou sur un appareil explicitement ciblé.

**Blocked by:** 11 — Vérifier le fonctionnement d’un script ; 13 — Initialiser et déployer un projet TOML ; 15 — Lancer et arrêter une cible headless.

**Status:** in-progress

## Acceptance criteria

- [ ] Un schéma déclaratif TOML documenté couvre script actif sans erreur, valeurs d’état et texte attendu dans les logs, avec fenêtres temporelles bornées et sémantique de « actif » explicitée.
- [ ] Le résultat expose les assertions réussies/échouées et les erreurs d’exécution ; le code de sortie est non nul pour un test échoué ou inexécutable.
- [ ] Par défaut, les tests créent une instance headless et des données isolées, déploient le projet puis exécutent les assertions. Deux exécutions ne partagent pas implicitement l’état persistant.
- [ ] Les tests sur appareil physique exigent une sélection explicite ; un profil global par défaut ne suffit pas à les déclencher.
- [ ] Les instances créées sont nettoyées après succès, échec et interruption. Une instance externe utilisée explicitement n’est pas arrêtée.
- [ ] Le runner n’intègre ni commandes arbitraires ni système de build ; les limites d’observation et de logs restent visibles.
- [ ] Aide et descriptions sont complètes ; les tests CLI et les tests avec véritable AWTRIX headless couvrent un projet script/module/ressource, réussite, assertion échouée, erreur Berry et isolation.

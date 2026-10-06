# 13: Initialiser et déployer un projet TOML

**What to build:** Initialiser, valider et déployer un projet de scripts comprenant scripts, modules, ressources et configuration dans un parcours additif.

**Blocked by:** 02 — Gérer les profils et sélectionner la cible ; 07 — Lire et déployer un script avec protection contre les conflits ; 08 — Gérer le cycle de vie et la configuration des scripts ; 12 — Gérer les modules Berry et les ressources.

**Status:** ready-for-agent

## Acceptance criteria

- [ ] L’initialisation produit un manifeste TOML minimal et un script valides. Le schéma, l’identité de projet et les exemples sont documentés et découvrables par description structurée.
- [ ] Les chemins sont relatifs au manifeste ; la validation détecte les entrées invalides et dépendances locales manquantes avant toute mutation distante.
- [ ] La référence de profil du projet s’insère dans la priorité commande, environnement, projet, défaut global. Le manifeste ne stocke pas les secrets.
- [ ] Scripts, modules, ressources et configuration sont déployés dans un ordre explicite tenant compte des dépendances. Les mises à jour de scripts conservent leurs protections contre les conflits.
- [ ] Le déploiement est additif : les éléments distants non déclarés ne sont pas supprimés.
- [ ] Au premier échec, les opérations suivantes s’arrêtent ; le rapport distingue réussies, échouées et non réalisées, sans promettre de transaction globale.
- [ ] Les tests CLI couvrent chemins relatifs, cible du projet, validation avant mutation, ordre des opérations, contenu étranger préservé et échec partiel. Un projet d’exemple script/module/ressource est déployable.

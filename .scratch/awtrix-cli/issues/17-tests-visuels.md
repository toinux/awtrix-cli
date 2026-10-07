# 17: Tester le rendu par comparaison de captures

**What to build:** Ajouter aux tests de projet des assertions de rendu basées sur une image de référence et une tolérance explicite.

**Blocked by:** 16 — Exécuter les tests déclaratifs d’un projet.

**Status:** done

## Acceptance criteria

- [ ] Le schéma déclare la référence et la tolérance avec une sémantique précise, documentée et validée, sans accepter silencieusement des dimensions incompatibles.
- [ ] Les références locales sont résolues selon les conventions du manifeste et les captures respectent les dimensions réelles de la cible.
- [ ] Le rapport expose résultat et mesures de comparaison ; les captures et artefacts utiles sont enregistrés dans des fichiers plutôt qu’inclus en pixels dans JSON.
- [ ] Les exemples utilisent un affichage contrôlé. L’aide explique les limites d’horloge, animations, réseau et corrections physiques, sans promettre de capture à une frame exacte.
- [ ] La description structurée expose les nouvelles assertions ; les erreurs de référence/image sont distinguées des différences de rendu.
- [ ] Les tests CLI et headless couvrent égalité, différence tolérée, dépassement de tolérance et dimensions incompatibles.

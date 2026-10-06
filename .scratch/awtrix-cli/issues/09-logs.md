# 09: Consulter et suivre les logs

**What to build:** Lire les logs et les suivre progressivement avec les curseurs de l’API, pour une utilisation humaine ou agent.

**Blocked by:** 01 — Identifier et diagnostiquer un appareil.

**Status:** in-progress

## Acceptance criteria

- [ ] La lecture et le polling utilisent le curseur retourné par l’API et permettent une reprise explicite.
- [ ] Le suivi fournit une sortie humaine ou JSON Lines, avec intervalle et durée bornée configurables ; les enregistrements et la fin du flux sont décrits.
- [ ] La sortie est progressive et stdout reste exploitable sans mélange de diagnostics auxiliaires.
- [ ] L’historique limité et les pertes éventuelles sont expliqués sans promettre une collecte exhaustive ; les préfixes de script peuvent servir au filtrage sans inventer un format de log absent.
- [ ] L’interruption et les erreurs réseau terminent le suivi proprement avec le résultat approprié.
- [ ] Les tests CLI vérifient progression des curseurs, absence de doublons causés par le polling, flux vide, durée et échec distant.

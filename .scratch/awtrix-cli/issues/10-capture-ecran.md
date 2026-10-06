# 10: Capturer l’écran dans un fichier

**What to build:** Récupérer le framebuffer d’une cible et produire une image inspectable sans déverser les pixels dans le contexte de l’agent.

**Blocked by:** 01 — Identifier et diagnostiquer un appareil.

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Une capture produit un fichier image, au minimum PNG, aux dimensions retournées par l’appareil, sans supposer une matrice ESP32 fixe.
- [ ] L’ordre et l’encodage RGB des pixels sont respectés ; une réponse incohérente ou tronquée est rejetée explicitement.
- [ ] Le résultat machine contient emplacement, format et dimensions, sans la matrice complète ; les erreurs d’écriture locale sont structurées.
- [ ] L’aide explique qu’un framebuffer ne reproduit pas luminosité et corrections physiques ni une synchronisation déterministe avec une frame.
- [ ] La description structurée expose arguments et résultat.
- [ ] Les tests CLI vérifient dimensions et pixels de l’image produite, les variantes et les erreurs réseau/fichier.

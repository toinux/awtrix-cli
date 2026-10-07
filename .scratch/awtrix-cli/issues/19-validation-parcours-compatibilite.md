# 19: Valider le parcours complet et la compatibilité annoncée

**What to build:** Démontrer le parcours agent complet et fournir une matrice de compatibilité fondée sur des résultats observés, avec distinction entre simulation HTTP, headless et matériel physique.

**Blocked by:** 03 — Afficher une progression avec une application poussée ; 04 — Envoyer et retirer des notifications ; 05 — Piloter les applications et la rotation ; 06 — Administrer l’affichage et redémarrer ; 14 — Suivre les éléments du projet et les retirer explicitement ; 17 — Tester le rendu par comparaison de captures ; 18 — Distribuer le binaire sur les trois systèmes hôtes.

**Status:** ready-for-agent

**Blocker:** Ticket 18 is not accepted: native macOS/Windows pipeline checks unavailable. Do not start final compatibility acceptance until those prerequisite checks pass. Physical target checks also require explicitly supplied hardware; unavailable physical results must remain marked not executed.

## Acceptance criteria

- [ ] Un scénario reproductible couvre découverte de cible, affichage de progression, déploiement d’un projet script/module/ressource, tests d’état/logs/rendu et notification finale, avec les conditions de nettoyage documentées.
- [ ] Un agent peut réaliser ce scénario à partir de l’aide et des descriptions intégrées ; les exemples correspondent aux commandes et schémas effectivement livrés.
- [ ] Le parcours de correction couvre une erreur Berry malgré HTTP réussi et une modification concurrente ; les rapports restent exploitables avec jq.
- [ ] Les contrats des trois variantes sont vérifiés et les versions testées/minimales vérifiées sont documentées, avec les capacités spécifiques et incompatibilités pertinentes.
- [ ] Le parcours principal est exécuté sur chaque variante physique disponible. Les validations matérielles indisponibles sont explicitement marquées comme non réalisées avec protocole de reprise ; aucune simulation ou exécution headless ne vaut validation physique.
- [ ] Les preuves de tests séparent simulation HTTP, véritable headless, appareil physique et système hôte. Le headless ne sert pas à certifier mémoire, instructions, audio ou capteurs ESP32.
- [ ] Les vérifications requises passent et les écarts à la spécification sont corrigés ou documentés explicitement ; aucun périmètre AWTRIX 3, MQTT ou administration exclue n’est introduit.

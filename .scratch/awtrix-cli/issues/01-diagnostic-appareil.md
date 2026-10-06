# 01: Identifier et diagnostiquer un appareil

**What to build:** Un premier binaire Rust permettant de joindre une cible HTTP explicite, de lire identité, état et capacités et de diagnostiquer les échecs. Il établit le contrat CLI commun à partir d’un parcours réellement utilisable.

**Blocked by:** None (can start immediately).

**Status:** done

## Acceptance criteria

- [ ] Le binaire propose aide, version, identité, état, capacités et diagnostic de connexion pour une cible HTTP explicite, avec authentification Basic optionnelle.
- [ ] Les contrats officiels des variantes ESP32, ESP32-S3 et TC002 sont vérifiés ; la variante et la version détectées sont exposées sans assimiler TC002 à ESP32.
- [ ] Les sorties humaines, le JSON compact et la sélection de champs fonctionnent indépendamment de la présence d’un terminal. Les champs inconnus produisent une erreur compréhensible.
- [ ] Les schémas de résultats et d’erreurs et les codes de sortie sont explicitement définis : résultats machine sur stdout, diagnostics auxiliaires sur stderr, échec non nul, codes d’erreur stables.
- [ ] Une description structurée consultable hors connexion expose paramètres, entrées, sorties, exemples et prérequis ; l’information connectée précise les capacités disponibles. ESP32 sert de référence hors connexion.
- [ ] Les délais réseau sont bornés ; les erreurs de transport, d’authentification, HTTP et d’incompatibilité sont distinguées sans exposer les secrets.
- [ ] Les tests exécutent le binaire contre un serveur HTTP de test pour les trois variantes, succès, authentification refusée, délai dépassé et réponse invalide. Ils vérifient sorties, codes de sortie et requêtes observées.

## Context

Tranche initiale de la spécification « CLI AWTRIX NG pour agents et développement Berry ». Chaque tranche suivante étend l’aide, la description structurée et les tests de contrat avec ses propres comportements. Pas de prise en charge d’AWTRIX 3 ou de MQTT.

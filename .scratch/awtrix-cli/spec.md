# CLI AWTRIX NG pour agents et développement Berry

Status: ready-for-agent

## Problem Statement

Un agent doit pouvoir développer, tester et administrer des appareils AWTRIX NG sans deviner les contrats HTTP ni dépendre d’une interface web. Aujourd’hui, les différences entre variantes, les erreurs Berry indépendantes du statut HTTP et les opérations de déploiement multiples rendent cette boucle difficile à automatiser correctement.

L’utilisateur souhaite un binaire autonome, également agréable à utiliser manuellement, qui permette de passer d’un source Berry à un résultat observé et testé. Il doit gérer plusieurs appareils et les variantes ESP32, ESP32-S3 et TC002, avec des sorties exploitables par des outils comme jq et économes en tokens.

## Solution

Construire un CLI Rust distribué sous forme de binaire autonome pour Linux, macOS et Windows. Le nom proposé est `awtrix`. La première version communique par HTTP avec AWTRIX NG exclusivement.

Le CLI offre des commandes élémentaires composables et des parcours de haut niveau pour déployer un projet de scripts, vérifier son fonctionnement et exécuter des tests. Son aide et ses descriptions structurées permettent à un agent de découvrir les commandes sans documentation injectée dans son contexte.

Une cible peut être un appareil physique ou une instance AWTRIX Linux headless. Les commandes adaptent leur comportement aux capacités réelles de la cible et expliquent les incompatibilités. Les tests locaux utilisent un environnement isolé ; les tests physiques exigent une cible explicite.

## User Stories

1. En tant qu’agent, je veux découvrir les commandes depuis le terminal afin de les utiliser sans supposer leur syntaxe.
2. En tant qu’agent, je veux consulter les schémas, exemples et prérequis d’une commande afin de construire une invocation valide.
3. En tant qu’agent, je veux consulter les descriptions hors connexion afin de préparer mon travail sans appareil disponible.
4. En tant qu’utilisateur, je veux installer un binaire autonome sur Linux, macOS ou Windows afin d’éviter la gestion d’un environnement d’exécution supplémentaire.
5. En tant qu’utilisateur, je veux une sortie lisible afin de piloter manuellement mon appareil.
6. En tant qu’agent, je veux du JSON compact afin de traiter les résultats de manière fiable et économe en tokens.
7. En tant qu’utilisateur, je veux exploiter les résultats avec jq afin de composer mes automatisations.
8. En tant qu’agent, je veux sélectionner les champs retournés afin de limiter les données inutiles.
9. En tant qu’agent, je veux des erreurs structurées et des codes de sortie stables afin de distinguer succès, conflit, incompatibilité et échec.
10. En tant qu’utilisateur, je veux créer et gérer des profils d’appareil nommés afin de piloter plusieurs écrans.
11. En tant qu’agent, je veux connaître la cible effectivement sélectionnée afin de vérifier où mes commandes s’exécutent.
12. En tant qu’utilisateur, je veux définir un appareil par défaut afin de simplifier mes commandes courantes.
13. En tant qu’utilisateur, je veux référencer un profil depuis un projet de scripts afin d’associer ce projet à sa cible de développement.
14. En tant qu’utilisateur, je veux conserver les identifiants hors du manifeste de projet afin de partager le projet sans partager ses secrets.
15. En tant qu’agent, je veux lire l’identité, la variante, la version et les capacités d’un appareil afin de choisir des opérations compatibles.
16. En tant qu’agent, je veux un diagnostic de connexion afin de comprendre les échecs de transport ou d’authentification.
17. En tant qu’agent, je veux lire l’état de l’appareil afin d’observer les effets de mes commandes.
18. En tant qu’agent, je veux créer et mettre à jour une application poussée afin d’afficher la progression de mon travail.
19. En tant qu’agent, je veux envoyer puis retirer une notification afin de signaler un événement sans laisser un affichage permanent.
20. En tant qu’utilisateur, je veux consulter et piloter les applications et la rotation afin de sélectionner l’affichage utile.
21. En tant qu’utilisateur, je veux régler la luminosité et l’alimentation de l’affichage afin de gérer son comportement quotidien.
22. En tant qu’agent, je veux capturer le rendu dans un fichier afin de vérifier l’affichage sans envoyer tous les pixels dans mon contexte.
23. En tant qu’agent, je veux lire le source d’un script distant afin de travailler à partir de son état réel.
24. En tant qu’agent, je veux déployer un fichier Berry isolé afin de disposer d’un parcours de développement simple.
25. En tant qu’agent, je veux détecter une modification concurrente du source afin de ne pas écraser silencieusement une correction faite ailleurs.
26. En tant qu’utilisateur, je veux demander explicitement un remplacement inconditionnel afin de résoudre les situations où je souhaite écraser le source distant.
27. En tant qu’agent, je veux distinguer source enregistré, démarrage vérifié et fonctionnement observé afin de ne pas confondre acceptation HTTP et réussite Berry.
28. En tant qu’agent, je veux activer, désactiver et supprimer des scripts afin de contrôler leur cycle de vie.
29. En tant qu’agent, je veux consulter et modifier la configuration des scripts afin de tester leur logique avec différents paramètres.
30. En tant qu’agent, je veux voir les erreurs Berry avec les informations disponibles de ligne et de hook afin de corriger le source.
31. En tant qu’agent, je veux suivre les logs en JSON Lines afin d’observer une exécution progressivement.
32. En tant qu’agent, je veux une vérification bornée dans le temps regroupant état, erreurs, logs et capture optionnelle afin de raccourcir ma boucle de correction.
33. En tant qu’utilisateur, je veux initialiser un manifeste et un script minimal valides afin de démarrer sans deviner les conventions.
34. En tant qu’agent, je veux valider un manifeste TOML avant le déploiement afin de détecter les erreurs locales.
35. En tant qu’agent, je veux déployer ensemble scripts, modules Berry, ressources et configuration afin de travailler sur un projet complet.
36. En tant qu’utilisateur, je veux un déploiement additif afin de conserver les éléments distants non déclarés dans mon projet.
37. En tant qu’utilisateur, je veux des suppressions explicites limitées aux éléments suivis par le projet afin de gérer son évolution.
38. En tant qu’agent, je veux un rapport d’échec partiel afin de savoir quelles opérations ont réellement été réalisées.
39. En tant qu’agent, je veux tester dans une instance headless isolée afin d’éviter l’influence de données persistées par un test précédent.
40. En tant qu’utilisateur, je veux réutiliser une instance headless persistante pour le développement interactif afin de conserver mon état de travail.
41. En tant qu’agent, je veux que le CLI lance et arrête un binaire AWTRIX Linux fourni ou configuré afin d’automatiser les tests locaux.
42. En tant qu’agent, je veux déclarer des assertions sur l’état des scripts et les logs afin de vérifier des comportements attendus.
43. En tant qu’agent, je veux comparer des captures avec une tolérance explicite afin de vérifier des rendus contrôlés.
44. En tant qu’utilisateur, je veux exécuter des tests sur un appareil physique explicitement choisi afin de vérifier son comportement réel.
45. En tant qu’agent, je veux connaître les limites du headless afin de ne pas conclure à tort à une validation matérielle.
46. En tant qu’agent, je veux gérer les fichiers et ressources nécessaires aux scripts afin de déployer leurs dépendances.
47. En tant qu’utilisateur, je veux lire la configuration système et modifier les réglages d’affichage afin d’administrer la cible de développement.
48. En tant qu’utilisateur, je veux redémarrer l’appareil afin de vérifier notamment les comportements persistants.
49. En tant qu’agent, je veux recevoir une incompatibilité explicite pour une fonction absente afin d’adapter mon parcours à la variante.

## Implementation Decisions

### Distribution et architecture

- Rust est retenu pour la distribution autonome et la robustesse des contrats typés, plutôt que pour accélérer les échanges réseau.
- La distribution vise Linux, macOS et Windows. La disponibilité de l’exécutable AWTRIX Linux headless reste une contrainte distincte : le CLI doit expliquer lorsqu’un environnement hôte ne peut pas le lancer.
- Les responsabilités à couvrir sont l’interface CLI et sa description structurée, la sélection des profils, la communication HTTP et les capacités, les opérations AWTRIX, le déploiement des projets et l’exécution des tests/headless. Leur découpage interne reste libre tant que les comportements externes sont respectés.
- Les domaines proposés sont appareil, profils, applications, notifications, scripts, projets, tests, logs, captures, fichiers, réglages et descriptions. La grammaire précise sera arrêtée dans les tranches d’implémentation correspondantes et restera cohérente entre domaines.

### HTTP et variantes

- HTTP est le seul transport de la première version. L’authentification HTTP Basic est prise en charge lorsque l’appareil la demande.
- Seul AWTRIX NG est couvert, avec ESP32, ESP32-S3 et TC002. TC002 ne doit pas être traité comme un ESP32.
- ESP32 est la référence par défaut pour l’aide hors connexion ; une cible connectée est identifiée par son identité, sa version et ses capacités.
- La sélection des fonctions repose sur les capacités réelles, pas uniquement sur le nom de variante. Les fonctions spécifiques restent accessibles lorsqu’elles sont disponibles.
- Les contrats officiels OpenAPI par variante servent de référence. Les versions minimales vérifiées seront documentées pendant l’implémentation ; aucune compatibilité historique non testée n’est promise.
- Les erreurs HTTP, erreurs Berry et échecs de transport sont distingués. Les limites applicables à la variante et à la route sont respectées, sans appliquer automatiquement la limite JSON au source Berry brut.
- Une notification acceptée ne doit pas être présentée comme nécessairement visible ; l’application sélectionnée dans la rotation ne prouve pas l’absence de notification à l’écran.
- Les opérations non idempotentes, notamment l’envoi de notifications, ne sont pas réessayées aveuglément après une réponse incertaine.

### Profils et sélection de cible

- La priorité est : sélection explicite dans la commande, variable d’environnement, cible du projet, profil global par défaut.
- Un diagnostic expose la cible effective et les informations de variante/version/capacités.
- Les secrets résident dans la configuration personnelle ou l’environnement ; le manifeste de projet référence un profil par son nom.

### Contrat pour les agents

- La sortie humaine est lisible par défaut ; un mode JSON uniforme fonctionne indépendamment de la présence d’un terminal.
- Le JSON machine est compact et compatible avec jq. Les commandes de suivi utilisent JSON Lines.
- Les résultats machine vont sur stdout ; les diagnostics auxiliaires vont sur stderr. Un échec produit un code de sortie non nul et une erreur machine comprenant code stable, contexte et détails utiles.
- Les listes et diagnostics sont synthétiques, avec détails sur demande et sélection de champs. La documentation doit permettre à l’agent de connaître ces champs.
- Les captures sont enregistrées dans des fichiers ; la réponse fournit notamment emplacement et dimensions plutôt que la matrice de pixels.
- Une description structurée, consultable par domaine ou commande, expose paramètres, entrées, sorties, exemples et prérequis de capacités. Elle fonctionne hors connexion et peut être affinée selon la cible connectée.
- Les codes de sortie, schémas de sortie et noms précis des options seront définis explicitement pendant l’implémentation et couverts par les tests de contrat.

### Scripts et déploiement protégé

- Un script Berry est persistant, contrairement à une application poussée. Les modules Berry sont importables sans constituer une application de rotation.
- Le CLI prend en charge lecture et déploiement du source, activation/désactivation, suppression, configuration, diagnostic des erreurs et suivi des logs.
- Le résultat distingue source enregistré, démarrage vérifié et fonctionnement observé sur une durée donnée. Un succès HTTP contenant une erreur Berry est un échec opérationnel, pas un déploiement fonctionnel réussi.
- Une désactivation globale de l’exécution des scripts doit empêcher de déclarer que le source sauvegardé a été compilé ou démarré.
- Une modification conditionnelle est utilisée par défaut lorsqu’elle est disponible, en comparant avec le source distant de référence. Un conflit est explicite ; le remplacement inconditionnel nécessite une demande explicite.
- Lorsque la protection n’est pas disponible, le CLI le signale et ne prétend pas fournir de garantie atomique par une simple lecture suivie d’une écriture.
- Les garanties de restauration après erreur de compilation ou de démarrage correspondent à celles de la route réellement utilisée. Elles ne sont pas étendues à des erreurs runtime ultérieures.
- Le suivi des logs respecte le curseur exposé par AWTRIX. L’historique étant limité, la vérification ne promet pas une collecte exhaustive de tous les messages.

### Projets de scripts

- Un manifeste TOML minimal déclare scripts, modules, ressources et configuration nécessaire. Les chemins locaux sont relatifs à l’emplacement du manifeste.
- Le fichier Berry isolé reste un parcours autonome, sans manifeste obligatoire.
- Des commandes d’initialisation produisent un manifeste et un script minimal valides.
- Le déploiement est additif par défaut : création et mise à jour des éléments déclarés sans suppression implicite des autres éléments distants.
- La suppression est explicite et limitée aux éléments précédemment suivis par ce projet. Le mécanisme de suivi doit permettre de distinguer ces éléments des autres contenus de l’appareil.
- À la première erreur, les opérations suivantes s’arrêtent et le rapport indique les opérations réussies, échouées et non réalisées. Aucune transaction globale entre scripts, configuration et ressources n’est promise.
- Le schéma TOML exact, l’identité de projet, le suivi des éléments et l’ordre des dépendances seront précisés dans la tranche projet sans modifier ces garanties.

### Vérification et tests utilisateur

- Les commandes élémentaires sont composables ; une vérification de haut niveau collecte état, erreurs, logs et éventuellement capture pendant une durée configurable et bornée.
- Les assertions initiales portent sur le script actif sans erreur, le texte attendu dans les logs, les valeurs d’état et une comparaison de capture avec tolérance explicite.
- Le format est déclaratif et limité ; les scénarios plus complexes composent le CLI depuis un programme externe.
- Les tests headless utilisent par défaut une instance dédiée et un répertoire de données isolé. Le CLI lance et arrête un binaire AWTRIX Linux fourni ou configuré.
- Une instance persistante peut être réutilisée pour le développement interactif. La fin ou l’échec d’un test doit nettoyer l’instance temporaire qu’il a créée, sans arrêter une instance externe réutilisée.
- Les tests physiques nécessitent une sélection explicite de cible.
- Les comparaisons visuelles concernent des affichages contrôlés. Le framebuffer ne représente pas la luminosité physique ni toutes les corrections des LEDs ; les animations, l’horloge et le réseau nécessitent des conditions de test adaptées.
- Le headless ne valide pas les capteurs, l’audio ni les budgets mémoire et instructions réels des appareils ESP32.

### Administration initiale

- Le périmètre comprend les fichiers et ressources utiles aux scripts, la configuration des scripts, les réglages d’affichage, la lecture de la configuration système et le redémarrage.
- Les opérations d’administration respectent les capacités et limites de chaque variante.

## Testing Decisions

La stratégie ci-dessous a été explicitement validée par l’utilisateur. Le dépôt ne contient actuellement ni code ni tests ; il n’y a donc pas de convention de test existante à réutiliser.

- La frontière principale est le binaire CLI : invocation avec arguments, environnement et fichiers, observation des sorties, codes de sortie et effets sur la cible. Les tests vérifient le comportement externe plutôt que l’organisation interne des modules.
- Un serveur HTTP de test permet de couvrir les contrats CLI de façon déterministe : priorité de sélection de cible, authentification, encodage des requêtes, JSON et sélection de champs, erreurs, réponses des trois variantes et capacités absentes.
- Les cas critiques incluent un HTTP réussi avec erreur Berry, un source enregistré sans exécution, un conflit de modification, une protection conditionnelle absente, une erreur runtime après démarrage, une réponse réseau incertaine et un déploiement partiellement réalisé.
- Les tests de projet vérifient les chemins relatifs, la validation du manifeste, le déploiement additif et la restriction des suppressions aux éléments suivis. Un contenu distant étranger au projet doit survivre au parcours de déploiement.
- Les tests de bout en bout utilisent un véritable AWTRIX headless pour déployer un script avec module et ressource, observer logs/état/capture et évaluer les assertions.
- Le parcours headless vérifie l’isolation entre exécutions, le lancement, l’arrêt et le nettoyage après succès ou échec. La réutilisation d’une instance externe ne transfère pas sa propriété au CLI.
- Une absence d’erreur pendant une fenêtre d’observation n’est pas assimilée à une preuve générale de correction du script.
- La validation physique reproduit le parcours principal sur ESP32, ESP32-S3 et TC002, en documentant les versions réellement testées. Une variante simulée par le serveur HTTP ou testée via headless ne doit pas être déclarée matériellement validée.
- La distribution vérifie que le binaire, l’aide et les opérations HTTP fonctionnent sur les systèmes hôtes visés. Le support d’exécution headless est annoncé séparément selon l’environnement compatible.

## Out of Scope

- AWTRIX 3 et les garanties de compatibilité avec des versions NG historiques non vérifiées.
- MQTT dans la première version.
- Écriture de la configuration réseau/MQTT, sauvegarde/restauration complète et mise à jour du firmware dans le périmètre initial.
- Téléchargement ou installation automatique du binaire AWTRIX headless.
- Un système de build généraliste ou un moteur de commandes arbitraires intégré au manifeste de tests.
- Débogueur Berry pas-à-pas, exécution déterministe d’un hook ou compilation distante sans déploiement lorsque l’API ne les fournit pas.
- Transaction globale de déploiement et restauration automatique de toutes les ressources.
- Émulation complète du matériel ou certification des budgets ESP32 depuis une instance Linux.
- Un troisième format machine propriétaire destiné à remplacer JSON.

## Further Notes

- La spécification reprend les décisions de l’interview ; les détails indiqués comme restant à préciser sont des contrats techniques à rendre explicites dans les tickets, et non des fonctionnalités supplémentaires à inventer.
- La réalisation est multi-session. L’étape suivante est un découpage en tickets tracer-bullet autonomes avec dépendances déclarées, donnant rapidement un parcours vertical appareil → commande → résultat machine → test.
- Le glossaire du dépôt est la référence de vocabulaire. Aucun prototype n’a été réalisé ; aucune validation sur appareil physique n’a encore été effectuée.
- Références officielles utilisées pendant la conception :
  - Documentation générale : https://blueforcer.github.io/awtrix-ng/
  - Contrat ESP32 : https://blueforcer.github.io/awtrix-ng/esp32/api/openapi.yaml
  - Contrat ESP32-S3 : https://blueforcer.github.io/awtrix-ng/esp32-s3/api/openapi.yaml
  - Contrat TC002 : https://blueforcer.github.io/awtrix-ng/tc002/api/openapi.yaml
  - API HTTP et scripts : https://blueforcer.github.io/awtrix-ng/esp32/reference/http/
  - Cycle de vie Berry : https://blueforcer.github.io/awtrix-ng/esp32/guides/scripting/
  - Développement Linux/headless : https://blueforcer.github.io/awtrix-ng/tc002/developers/linux/
- Les contrats exacts doivent être revérifiés contre les versions choisies lors de l’implémentation, notamment pour les mises à jour conditionnelles, les limites et les différences de variante.

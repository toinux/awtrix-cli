# Pilotage AWTRIX

Vocabulaire du pilotage, du développement de scripts et de l’administration d’appareils AWTRIX par un agent ou une personne.

## Langage

**Appareil** :
Une instance AWTRIX que l’on peut piloter et dont on peut consulter l’état.

**Variante** :
La famille de plateforme d’un appareil AWTRIX : ESP32, ESP32-S3 ou TC002. Ses capacités peuvent différer de celles des autres variantes.

**Profil d’appareil** :
Une désignation locale d’un appareil, associée aux informations nécessaires pour le joindre. Un projet peut référencer ce profil par son nom.

**Application poussée** :
Un contenu d’affichage calculé à l’extérieur de l’appareil et participant à sa rotation. Ce contenu est temporaire et disparaît au redémarrage.

**Notification** :
Un affichage temporaire qui peut interrompre la rotation ou attendre dans une file.

**Script** :
Un programme Berry conservé sur l’appareil, exécutant sa propre logique et pouvant produire un affichage.

**Projet de scripts** :
Un ensemble déclaré de scripts, modules et ressources destinés à être déployés et testés ensemble.

**Module Berry** :
Du code Berry conservé sur l’appareil et importable par un script, sans constituer lui-même une application de la rotation.

**Rotation** :
La succession des applications présentées sur l’écran de l’appareil.

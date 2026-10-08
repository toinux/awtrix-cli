# Pilotage AWTRIX

Vocabulaire du pilotage, du développement de scripts et de l'administration d'appareils AWTRIX par un agent ou une personne.

## Langage

**Appareil** :
Une instance AWTRIX que l'on peut piloter et dont on peut consulter l'etat.
_Avoid_: Device, instance

**Variante** :
La famille de plateforme d'un appareil AWTRIX : ESP32, ESP32-S3 ou TC002. Ses capacites peuvent differer de celles des autres variantes.
_Avoid_: Platform, board type

**Profil d'appareil** :
Une designation locale d'un appareil, associee aux informations necessaires pour le joindre. Un projet peut referencer ce profil par son nom.
_Avoid_: Device profile, target, host config

**Application poussee** :
Un contenu d'affichage calcule a l'exterieur de l'appareil et participant a sa rotation. Ce contenu est temporaire et disparait au redemarrage.
_Avoid_: Pushed app, external app

**Notification** :
Un affichage temporaire qui peut interrompre la rotation ou attendre dans une file.
_Avoid_: Alert, toast

**Script** :
Un programme Berry conserve sur l'appareil, executant sa propre logique et pouvant produire un affichage.
_Avoid_: Berry script, app script

**Minification** :
Transformation d'un script visant a reduire sa taille tout en conservant son comportement et ses metadonnees AWTRIX.
_Avoid_: Compression, minify (pour designer le resultat)

**Projet de scripts** :
Un ensemble declare de scripts, modules et ressources destines a etre deployes et testes ensemble.
_Avoid_: Script project, bundle

**Module Berry** :
Du code Berry conserve sur l'appareil et importable par un script, sans constituer lui-meme une application de la rotation.
_Avoid_: Berry module, library

**Rotation** :
La succession des applications presentees sur l'ecran de l'appareil.
_Avoid_: App rotation, carousel

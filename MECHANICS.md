# Règles reproduites

L’exécutable natif est un lanceur. Son entrée `0x402fef` rejoint `0x401700` et ses chaînes incluent `java.exe` et `javaw.exe`. Les fonctions métier sont dans l’archive Java de l’installation. Les adresses ci-dessous sont des offsets d’instructions dans chaque méthode Java. Les branchements ambigus du pseudo-code ont été lus dans le bytecode.

| Règle | Valeur | Repère |
| --- | --- | --- |
| Entrées ET et NON-ET | 4 broches | VariableConnectorBaseBlock.initInConnectors @ 0x0019 (profil à quatre entrées) |
| ET et NON-ET sans fil | true | AndBlock.getDefaultInConnectorValue @ 0x0000 |
| Entrées OU et NON-OU | 4 broches | VariableConnectorBaseBlock.initInConnectors @ 0x0019 (profil à quatre entrées) |
| OU et NON-OU sans fil | false | OrBlock.getDefaultInConnectorValue @ 0x0000 |
| Entrée NON sans fil | true | NotBlock.initInConnectors @ 0x0012 |
| Entrées OU exclusif | 2 broches | XorBlock.initInConnectors @ 0x0001 |
| ET | toutes les entrées sont à 1 | AndBlock.calculateBinaryValue @ 0x0019 |
| OU | au moins une entrée à 1 | OrBlock.calculateBinaryValue @ 0x0019 |
| NON | inverse l’entrée | NotBlock.calculateBinaryValue @ 0x000c |
| NON-ET | inverse le résultat ET | NandBlock.calculateBinaryValue @ 0x0001 |
| NON-OU | inverse le résultat OU | NorBlock.calculateBinaryValue @ 0x0001 |
| OU exclusif | les deux entrées diffèrent | XorBlock.calculateBinaryValue @ 0x0019 |
| Entrée binaire au départ | false | InputBlock.calculateBinaryValue @ 0x0029 |
| Sortie binaire | reproduit l’entrée câblée | OutputBlock.endCalculation @ 0x0009 |
| Retard à la montée | activation après maintien du déclencheur pendant T, extinction immédiate au relâchement | OnDelayParameter.calculateValue @ 0x0000, 0x0022, 0x0046; Timer.isFinished @ 0x0012 |
| Retard à la descente | activation immédiate, maintien pendant T après la descente, remise à zéro prioritaire | OffDelayParameter.calculateValue @ 0x0000, 0x002d, 0x0051; Timer.isFinished @ 0x0012 |
| Durée initiale des temporisations | 0 s | TimeProtectionParameter.getTimer @ 0x0008; Timer.<init>() @ 0x0009 |
| Relais mémorisé | R met à zéro, S met à un, sinon l’état est conservé | LatchingRelayParameter.getValue @ 0x0000-0x002b |
| Compteur binaire | un pas sur un front montant de Cnt, Dir 0 monte, Dir 1 descend, R remet à zéro | CounterParameter.calculateValueLimes @ 0x0000-0x0037; CounterParameter.updateCount @ 0x0000-0x0041 |
| Sortie du compteur à seuil simple | le compte atteint ou dépasse le seuil, sauf pendant la remise à zéro | CounterParameter.calculateValueLimes @ 0x0016-0x0037 |
| Minimum du compteur | 0 impulsions | CounterParameter.<init>(Block) @ 0x000a |
| Maximum du compteur | 99999 impulsions | CounterParameter.<init>(Block) @ 0x000f |
| Seuil initial du compteur | 3 impulsions | Estimé pour cette version |
| Cadence de rafraîchissement visée | 20 ms | Estimé pour cette version |
| Grille visible au départ | false | Application.ini, DE.siemens.ad.pdraw.app.LogoComfort.grid.visibility |
| Démarrage immédiat à l’entrée en simulation | true | Application.ini, simulation.startAutomatically |

La durée initiale vient du constructeur du temporisateur sans profil matériel. Le seuil initial est un choix d’édition après examen du constructeur et du mode à seuil simple. La cadence locale sert au rafraîchissement de la simulation. Les temporisateurs utilisent le temps écoulé. Le projet enregistre le schéma et les paramètres. L’état de simulation est remis à zéro à chaque démarrage.

## Comment LOGO!Soft Comfort lit les E/S en test en ligne

Le LOGO! exécute le programme en mode RUN. Le PC affiche les états et les valeurs issus de l’automate dans le schéma correspondant. Siemens demande que le programme ouvert et celui de l’automate soient identiques. [Aide Siemens, Test en ligne](https://cache.industry.siemens.com/dl/files/807/109974807/att_1299328/v4/Help_en-US_en-US.pdf#page=46)

Dans le logiciel installé, le chemin TCP de LOGO! 8 interroge l’automate avec plusieurs commandes Siemens. `GetPIB` lit l’image de processus où figurent les E/S, `GetFB` lit les résultats des blocs et `GetAVB` demande les valeurs courantes des blocs sélectionnés. Le logiciel recoupe le contrôle d’intégrité du programme, puis associe les valeurs de l’image de processus aux blocs du schéma. Une pause de 100 ms apparaît après les échanges dans cette boucle TCP, à laquelle s’ajoute le temps des requêtes.

Ce fonctionnement vient de l’analyse statique du fichier installé `lib/classes.jar`. Une capture réseau sur automate reste à faire pour mesurer la cadence et confirmer le transport réellement choisi pour chaque profil. Le suivi E/S d’Atelier logique lit les bits I et Q en Modbus TCP. Les menus S7 et Modbus des appareils configurent les liaisons de la simulation locale.

- Requêtes et pause dans `OnlineTestProtocolHandler$ConnectedTesting` (`ev_a4c66912485642e90aa07fff1681cb6375ff16d65093aed82916bb3ecb070acc`, `ev_7972e0a77a0b75a98c140ddb5545e8045a38e9241807577e4ca5b6a3615df85d`).
- Association des E/S dans `OnlineTestInterpreter$ValueDispatcher` (`ev_6f6bf42b900c9281dd8a5fed73d46f013c4abec4667c29ad0679d794baf0da21`, `ev_e37ed8e9b1db708fb25f73e9fefc2535920846cc497f9ed419036ed88890c6c2`).

## Paramètres de simulation locale

Les fonctions utilisateur incorporent un schéma Atelier logique avec jusqu’à quatre entrées numériques et une sortie numérique. Les conversions flottantes partagent la mémoire VM de la simulation et les archives conservent les derniers relevés jusqu’à la fin de la simulation. Les programmations hebdomadaire et annuelle utilisent un décalage UTC fixe saisi dans leurs propriétés. L’horloge astronomique calcule des heures approximatives de lever et de coucher à partir de la position saisie. [Aide Siemens, Fonctions spéciales](https://cache.industry.siemens.com/dl/files/807/109974807/att_1299328/v4/Help_en-US_en-US.pdf#page=284) · [Équations de position solaire de la NOAA](https://gml.noaa.gov/grad/solcalc/solareqns.PDF)

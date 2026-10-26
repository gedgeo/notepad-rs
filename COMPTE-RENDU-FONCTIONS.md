# Fonctions de pages — 26 octobre 2026

## Livré

Les options de page se trouvent dans le panneau droit. La navigation et le renommage restent dans le panneau gauche.

| Fonction | Comportement disponible |
| --- | --- |
| Liens et backlinks | Insertion de liens à identifiant stable `[[id:identifiant\|Nom]]`, navigation et références entrantes. Les anciens liens par nom sont résolus par les noms et alias conservés après renommage. |
| Favoris et épinglées | Deux indicateurs persistants et une liste d'accès rapide. |
| Corbeille | Masquage dans la liste de pages et restauration sans suppression des données. |
| Blocs | Mode « Éditeur visuel de blocs » : édition séparée, ajout, suppression et réorganisation ; titres agrandis, tâches à cocher et tableaux éditables cellule par cellule. Le Markdown reste le format d'échange, et le mode texte reste disponible. |
| Glisser-déposer | Réorganisation et déplacement sous une page parente, contrôle des cycles et retour à la racine. |
| Icônes et couleurs | Personnalisation persistante de l'icône dans la navigation. |
| Export | Markdown, HTML mis en forme et PDF direct avec polices Arial embarquées, accents et pagination, sans boîte d'impression. |
| Versions | Versions manuelles, captures espacées lors de l'édition, restauration ; conservation des 50 dernières captures automatiques. |
| Sauvegarde et échange | Export/import JSON avec images embarquées, copie .bak avant remplacement. Synchronisation automatique optionnelle toutes les cinq secondes par dossier partagé ; comparaison avec la dernière version commune, mises à jour et copies de conflit en cas d'édition concurrente. |

## Corrections des ajouts précédents

- Mise à jour vers l'API Panel d'egui 0.37.
- Initialisation des champs manquants qui empêchaient la compilation.
- Sauvegarde et restauration réelles des noms, identifiants et parents des pages.
- Conservation des pages vierges et de leur texte dans la session.
- Réattribution des identifiants dupliqués dans les anciennes sessions.
- Marquage des modifications du moodboard pour l'autosauvegarde.

## Vérification

93 tests Rust réussis après la refonte d'interface et compilation de développement réussie via `build.ps1` (WinLibs). Les tests vérifient le PDF avec accents, les liens après renommage, les noms ambigus, la corbeille, la restauration d'images après suppression de l'original, les conflits de synchronisation, le verrou du dossier, l'échange de deux instances simulées, la nouvelle navigation et la mise en forme des blocs.

Les tests egui simulent les clics et le déplacement du moodboard, sa duplication/suppression et sa restauration. Ils vérifient aussi la tâche à cocher en mode blocs et l'absence de réécriture du document lors du simple affichage.

Les vues texte, blocs et moodboard ont été capturées avec le moteur natif et comparées aux maquettes validées. Une petite fenêtre et le thème sombre disposent également de captures. Le service de contrôle Windows externe reste bloqué par MXC : les gestes sont testés par événements egui et les captures ne remplacent pas une vérification manuelle des dialogues Windows. Voir `design-qa.md` et `INTERFACE.md`.

## Refonte d'interface

- Navigation repliable avec recherche, favoris, pages hiérarchiques et moodboards.
- Document centré, titres lisibles et outils contextuels plutôt que deux panneaux permanents.
- Inspecteur des blocs et des éléments sélectionnés, aperçu des images et outils avant/arrière.
- Canevas de moodboard avec grille, zoom, déplacement du fond, ajustement au contenu et poignée de redimensionnement.
- Échelle de l'interface réglable et mémorisée ; navigation et propriétés superposées pour les petites fenêtres.
- Les fonctions historiques restent disponibles dans les menus et les paramètres.

## Utilisation des ajouts de finalisation

1. Dans le panneau droit, cocher « Éditeur visuel de blocs » pour basculer l'éditeur.
2. Utiliser « Insérer » dans la liste des pages pour créer un lien stable résistant au renommage. Les cibles ambiguës saisies à la main ne sont pas ouvertes arbitrairement.
3. Dans « Exporter », choisir « Exporter .pdf… » pour générer directement un PDF.
4. Dans « Sauvegarde », exporter l'espace de travail pour produire un JSON portable avec les images. Une image manquante provoque une erreur explicite plutôt qu'une sauvegarde incomplète.
5. Pour la synchronisation, choisir « Activer la synchronisation automatique… » et sélectionner le même dossier partagé sur les machines concernées. Les copies « — conflit » se trouvent parmi les pages et doivent être réconciliées par l'utilisateur.

## Limites explicites

- La synchronisation porte sur un fichier d'espace de travail, pas sur un service cloud intégré ni une coédition en temps réel. Le transport du dossier entre ordinateurs dépend du partage réseau ou de son logiciel de synchronisation. Les verrous de fichiers n'offrent pas de verrouillage distribué immédiat sur des copies OneDrive distinctes.
- Une fermeture brutale pendant la synchronisation peut laisser `.notepad-rs-sync.lock` : vérifier qu'aucune instance n'écrit avant de retirer ce fichier. Aucun verrou d'un autre processus n'est supprimé automatiquement.
- Le PDF exporte les pages texte. Les glyphes absents d'Arial peuvent provoquer une erreur explicite ; l'ancien export n'est remplacé qu'après un rendu réussi. Le PDF ne reproduit pas le canevas d'un moodboard.
- L'éditeur visuel utilise un bloc par groupe de lignes séparé par une ligne vide et préserve les blocs de code clôturés. Ce n'est pas une réplique de toutes les possibilités de blocs de Notion.
- Une restauration manuelle ajoute des copies avec de nouveaux identifiants ; elle ne remplace pas les pages ouvertes. Les images embarquées sont reconstituées dans le dossier local des pièces jointes. Limites : 32 Mo par image à l'export, 128 Mo pour un fichier importé.
- La corbeille conserve les onglets et leurs données ; elle ne supprime pas les fichiers sur disque.

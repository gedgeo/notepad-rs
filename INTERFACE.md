# Interface native Notepad RS

La refonte reprend les trois vues validées : écriture, blocs et moodboard. L'application reste en Rust/egui ; aucun site ni navigateur embarqué n'a été ajouté.

## Utilisation

- La colonne gauche contient la recherche, la création de pages, les favoris, l'arborescence, les moodboards, la corbeille et les paramètres.
- Un clic sur une ligne ouvre la page ; double-clic ou F2 permet de la renommer. Le menu contextuel propose sous-page, favoris et corbeille.
- Le bouton de navigation en haut permet de replier la colonne. En petite fenêtre, la navigation et les propriétés utilisent des fenêtres superposées.
- Texte et Blocs basculent le mode de la page. Les blocs possèdent un menu de déplacement/suppression et une barre contextuelle gras/italique/lien. Ces commandes modifient le Markdown du bloc.
- L'inspecteur s'ouvre pour le bloc ou l'élément de moodboard sélectionné. Il permet de régler les dimensions, couleur des notes, lien, duplication, suppression et ordre avant/arrière.
- Dans le moodboard, le pointeur déplace les éléments, la main déplace le fond. Les commandes de zoom et d'ajustement se trouvent au bas du canevas. La poignée du coin inférieur droit redimensionne l'élément sélectionné.
- Exporter propose les formats adaptés : Markdown/HTML/PDF pour une page et JSON avec images embarquées pour un moodboard.
- Le menu « … » conserve Fichier, Édition, Affichage et Aide, les onglets ouverts, les propriétés, l'historique et l'échelle de l'interface.
- Les commandes avancées de sauvegarde et synchronisation restent dans Paramètres. Les informations d'encodage, fins de ligne, curseur et sélection sont accessibles par Détails dans la barre d'état.

## Échelle et thèmes

L'échelle par défaut est de 125 %, avec choix 100/125/150 % dans le menu du document. Elle est enregistrée avec la visibilité et la largeur de navigation. Le mode clair est utilisé pour une nouvelle session ; les préférences de thème des sessions existantes sont conservées.

Les titres utilisent Segoe UI Bold quand elle est disponible et les icônes viennent de Phosphor, distribué sous licence MIT. Les polices Windows sont chargées depuis leur installation locale, sans être redistribuées.

## Vérification et captures

Lancer les tests et l'application avec le wrapper WinLibs :

```powershell
.\build.ps1 test
.\build.ps1 run
```

Les images `samples/previews/*-final.png` sont de vraies captures du moteur natif, avec des pages d'exemple isolées. Les photos d'exemple ne sont jamais insérées dans les sessions utilisateur.

Pour reproduire une capture depuis le répertoire du projet :

```powershell
target\x86_64-pc-windows-gnu\debug\notepad-rs.exe --design-preview=notes --capture-to=samples/previews/notes-final.png
```

Modes de vérification : `notes`, `blocks`, `moodboard`, `notes-narrow`. Le mode de capture ne sauvegarde pas la session de démonstration et se ferme après la capture. Le dossier `samples/design-assets` doit être présent pour reproduire le moodboard d'exemple.

Les comparaisons sont générées par `tools/compare_ui.py` (Pillow requis uniquement pour cet outil de développement). L'évaluation est consignée dans `design-qa.md`.

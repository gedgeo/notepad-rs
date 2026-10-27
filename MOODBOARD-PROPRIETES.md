# Propriétés du moodboard — 27 octobre 2026

Le panneau droit s’ouvre à l’entrée dans un moodboard. Le bouton « Propriétés du moodboard » permet de le rouvrir sans sélectionner un élément.

- **Canevas** : format libre, carré, paysage, portrait, A4, dimensions personnalisées, permutation largeur/hauteur, couleur de fond et visibilité du cadre.
- **Grille et alignement** : points ou lignes, espacement, visibilité, magnétisme des déplacements et réinitialisation de l’affichage. Ces valeurs sont propres au moodboard ; sans réglage local elles héritent des préférences de l’application.
- **Vue** : zoom, ajustement au canevas, retour à 100 % et outils de sélection/déplacement de vue.
- **Calques et ajout** : liste des notes/images, sélection, masquage, verrouillage, création d’une note et import d’image.
- **Export** : PNG, JPEG ou JSON portable avec les commandes existantes. La couleur de fond personnalisée est conservée dans les exports d’image ; la grille est un repère d’édition.
- **Élément sélectionné** : duplication/suppression en tête, aperçu, verrouillage, masquage, proportions, position, dimensions, alignement aux bords/centres, adaptation proportionnelle au canevas, couleur et texte des notes, lien, remplacement d’image et ordre avant/arrière.

Les éléments hors cadre sont conservés après un changement de taille. L’alignement aux bords nécessite un canevas de dimensions fixes. La petite fenêtre utilise des propriétés défilantes au premier plan pour éviter leur recouvrement par les images.

Validation : 132 tests réussis et compilation de développement réussie. Les cinq nouveaux tests vérifient la disponibilité sans sélection, la permutation non destructive, la persistance isolée par moodboard, les anciennes pages et la couleur de fond à l’export.

Mise à jour : panneau permanent à droite, redimensionnable et défilant, même dans une petite fenêtre. Toutes les sections sont ouvertes sans accordéon. Les propriétés de la sélection précèdent celles du canevas. Les réglages s'appliquent immédiatement.


## Catalogue et navigation des commandes

Le sommaire fixe en haut du panneau rejoint Élément sélectionné, Canevas, Grille et alignement, Vue, Calques et ajout et Export. Les sections restent ouvertes. Les sauts sont stabilisés pendant le chargement des aperçus d’image.

Les menus Fichier, Édition, Page, Insertion, Affichage, Outils et Aide partagent le catalogue `src/commands.rs` avec la palette. La palette est accessible par le bouton Rechercher une commande et Ctrl+Maj+P ; elle affiche les conditions des options indisponibles et accepte les flèches puis Entrée. Les réglages détaillés ouvrent leur écran existant.

Validation du 27 octobre 2026 : 138 tests réussis ; compilation de développement réussie. Tests ajoutés : identifiants uniques et conditions texte/note/image, protection en lecture seule, ajout/duplication/suppression et persistance, accès aux réglages, accès aux six sections, ouverture des sept menus dans les trois contextes. Captures natives vérifiées en 1440 × 980 et 640 × 700. Les dialogues système d’impression et de sélection de fichiers n’ont pas été exercés automatiquement.

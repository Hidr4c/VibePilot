# Procédure de Gestion des Branches GitHub - VibePilot

Ce document décrit le modèle de branching et la procédure technique pour mettre à jour la branche `draft` (Release Candidate) et publier des releases.

---

## 📌 Modèle de Branching

Le projet maintient une distinction claire entre le développement local et les branches publiées sur GitHub :

| Branche | Rôle | Commits |
| :--- | :--- | :--- |
| **`dev`** *(Local)* | Branche de travail active. | Historique complet (multiples commits de dev). |
| **`draft`** *(GitHub)* | Future release candidate. | **1 seul commit** au-dessus de la dernière release. |
| **`latest`** *(GitHub)* | Version de production stable la plus récente. | Alignée sur le tag de la dernière release. |
| **`release/vX.Y.Z`** *(GitHub)* | Archive de chaque version publiée (ex: `release/v1.0.1`). | **1 seul commit** représentant l'état de la version. |

> [!IMPORTANT]
> **Règle d'or sur GitHub** : Seules les branches `latest`, `draft` et les branches d'archives `release/vX.Y.Z` doivent être présentes sur le serveur distant. La branche de développement locale `dev` ne doit pas être poussée sur GitHub.

---

## 🔄 Procédure : Mettre à jour `draft` à partir de `dev`

Lorsque vous avez finalisé des fonctionnalités sur `dev` et souhaitez mettre à jour la branche `draft` sur GitHub sous forme d'un commit unique (squashé) au-dessus de la dernière release :

### Étape 1 : Préparer et nettoyer
Assurez-vous d'être sur `dev`, que tous les changements sont commités et que les tests passent :
```bash
git checkout dev
git status
cargo test
```

### Étape 2 : Réinitialiser la branche `draft`
Basculez sur la branche `draft` et réinitialisez-la sur le tag de la dernière release active.

Pour obtenir dynamiquement le dernier tag et réinitialiser la branche :

**Sur Linux / macOS (Bash/Zsh) :**
```bash
git checkout draft
LATEST_TAG=$(git tag --sort=-v:refname | head -n 1)
git reset --hard $LATEST_TAG
```

**Sur Windows (PowerShell) :**
```powershell
git checkout draft
$LatestTag = (git tag --sort=-v:refname)[0]
git reset --hard $LatestTag
```

*Note : Si vous préférez le faire manuellement, vous pouvez lister les versions avec `git tag` et exécuter `git reset --hard <tag>`.*

### Étape 3 : Importer le contenu de `dev`
Mettez à jour le répertoire de travail et l'index de git avec le code de `dev` :
```bash
git checkout dev -- .
```

### Étape 4 : Gérer les suppressions de fichiers
Si des fichiers ont été supprimés sur `dev` par rapport à la version de départ, ils doivent être supprimés de l'index de `draft`.
1. Listez les fichiers supprimés (marqués par un `D`) :
   ```bash
   git diff --name-status HEAD dev
   ```
2. Supprimez-les de l'index de `draft` (par exemple) :
   ```bash
   git rm src/config.rs src/orchestrator.rs ...
   ```

### Étape 5 : Créer le commit unique
Créez le commit unique décrivant toutes les nouveautés de cette future version :
```bash
git commit -m "feat: description des nouveautés et corrections"
```

### Étape 6 : Pousser sur GitHub
Poussez la branche `draft` mise à jour sur le serveur distant en forçant la mise à jour de l'historique :
```bash
git push origin draft --force
```

### Étape 7 : Retourner sur `dev`
Revenez sur votre branche locale de développement pour continuer à travailler :
```bash
git checkout dev
```

---

## 🚀 Procédure : Publier une Release officielle (ex: `v1.0.2`)

Une fois que la branche `draft` est validée et prête à être publiée :

### Étape 1 : Créer la branche de release
Créez une branche d'archive de release à partir de `draft` :
```bash
git checkout -b release/v1.0.2 draft
```

### Étape 2 : Poser le tag de version
Posez un tag annoté sur le commit pour figer la version :
```bash
git tag -a v1.0.2 -m "Release v1.0.2"
```

### Étape 3 : Aligner la branche `latest`
Mettez à jour la branche `latest` locale pour qu'elle pointe sur la nouvelle version :
```bash
git checkout latest
git reset --hard release/v1.0.2
```

### Étape 4 : Pousser les modifications sur GitHub
Envoyez la branche de release, le tag et la branche `latest` mise à jour sur GitHub :
```bash
git push origin release/v1.0.2
git push origin v1.0.2
git push origin latest --force
```

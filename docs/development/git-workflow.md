# Git Workflow & Deployment Strategy

Recommended Git branching strategy optimized for staging and production deployments. Adapt this workflow to fit your team's needs.

## Branch Structure

- **`main`** → Production environment
  - Protected branch
  - Deployed to production servers
  - Only accepts merges from `staging` after QA approval
  - Requires pull request reviews

- **`staging`** → Staging environment
  - Protected branch
  - Deployed to staging servers for QA testing
  - Accepts merges from `develop`
  - Used for final testing before production

- **`develop`** → Development integration branch
  - Integration branch for all features
  - Accepts merges from feature branches
  - Automatically deployed to development environment (optional)

- **`feature/*`** → Feature development branches
  - Created from `develop`
  - Naming convention: `feature/description-of-feature`
  - Merged back to `develop` via pull request

- **`bugfix/*`** → Bug fix branches
  - Created from `develop`
  - Naming convention: `bugfix/description-of-bug`
  - Merged back to `develop` via pull request

- **`hotfix/*`** → Emergency production fixes
  - Created from `main`
  - Naming convention: `hotfix/description-of-issue`
  - Merged to both `main` and `develop`
  - Used only for critical production issues

---

## Development Workflow

### 1. Starting a New Feature

```bash
git checkout develop
git pull origin develop
git checkout -b feature/my-new-feature
```

### 2. Committing and Pushing

```bash
git add .
git commit -m "feat: description of changes"
git push -u origin feature/my-new-feature
```

### 3. Creating a Pull Request

- Create PR from `feature/my-new-feature` → `develop`
- Request code review from team members
- Ensure all tests pass
- Merge after approval

### 4. Deploying to Staging

```bash
git checkout staging
git pull origin staging
git merge develop
git push origin staging
```

Staging environment will automatically deploy.

### 5. Deploying to Production

```bash
git checkout main
git pull origin main
git merge staging
git push origin main
```

Production environment will automatically deploy. Only after QA approval on staging.

---

## Hotfix Workflow

For critical production bugs:

```bash
# Create hotfix from main
git checkout main
git pull origin main
git checkout -b hotfix/critical-bug-fix

# Make changes and commit
git add .
git commit -m "hotfix: fix critical bug"

# Merge to main
git checkout main
git merge hotfix/critical-bug-fix
git push origin main

# Also merge to develop to keep in sync
git checkout develop
git merge hotfix/critical-bug-fix
git push origin develop

# Delete hotfix branch
git branch -d hotfix/critical-bug-fix
```

---

## Branch Protection Rules (Recommended)

**Note:** Branch protection rules require GitHub Pro or public repository. If using GitHub Free with private repositories, these rules must be enforced manually through team discipline.

### For `main` (Production):
- Require pull request reviews (minimum 1-2 approvers)
- Require status checks to pass before merging
- Require branches to be up to date before merging
- Do not allow force pushes
- Do not allow deletions

### For `staging`:
- Require pull request reviews (minimum 1 approver)
- Require status checks to pass before merging
- Do not allow force pushes

### For `develop`:
- Require status checks to pass before merging
- Allow squash and merge
- Do not allow force pushes

---

## Commit Message Convention

Follow conventional commits format:

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `style:` - Code style changes (formatting, etc.)
- `refactor:` - Code refactoring
- `test:` - Adding or updating tests
- `chore:` - Maintenance tasks

**Examples:**
```
feat: add user authentication endpoint
fix: resolve database connection timeout
docs: update API documentation
```

---

## Visual Flow Diagram

```
feature/xxx ──┐
feature/yyy ──┼──> develop ──> staging ──> main (production)
bugfix/zzz ──┘                    ↑
                                  │
hotfix/aaa ───────────────────────┴──> develop
```

---

## Best Practices

1. **Never commit directly to `main`, `staging`, or `develop`**
   - Always use feature/bugfix branches
   - Always create pull requests

2. **Keep branches up to date**
   ```bash
   git checkout develop
   git pull origin develop
   git checkout feature/my-feature
   git merge develop
   ```

3. **Write meaningful commit messages**
   - Use conventional commits format
   - Be descriptive about what changed and why

4. **Delete branches after merging**
   ```bash
   git branch -d feature/my-feature
   git push origin --delete feature/my-feature
   ```

5. **Sync hotfixes to both main and develop**
   - Ensures all branches stay in sync
   - Prevents regression in future releases

---

## Troubleshooting

### Merge Conflicts

```bash
# Update your branch with latest develop
git checkout feature/my-feature
git fetch origin
git merge origin/develop

# Resolve conflicts in your editor
# Then:
git add .
git commit -m "fix: resolve merge conflicts"
git push
```

### Accidentally Committed to develop

```bash
# Create a new branch from current commit
git branch feature/accidental-commit

# Reset develop to origin
git reset --hard origin/develop

# Switch to new branch
git checkout feature/accidental-commit

# Push new branch
git push -u origin feature/accidental-commit
```

### Need to Update PR with Latest develop

```bash
git checkout feature/my-feature
git fetch origin
git merge origin/develop
git push
```

---

## Simplified Workflow (Alternative)

For smaller teams or simpler projects, you can use a simplified workflow:

- **`main`** → Production (protected)
- **`feature/*`** → Feature branches created from and merged to `main`

```bash
# Start feature
git checkout main && git pull
git checkout -b feature/my-feature

# Complete feature
git push -u origin feature/my-feature
# Create PR to main, merge after review
```

---

For more information on Git best practices, see the [official Git documentation](https://git-scm.com/doc).

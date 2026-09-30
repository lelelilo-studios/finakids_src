#!/usr/bin/env bash
# Publishes a site directory to the public repo's main branch (served by GitHub Pages).
# usage: deploy_site.sh <site_dir> [--keep-downloads]
#   --keep-downloads  update only the web files, keeping downloads/ from the current site
# Authentication: SSH deploy key in $SITE_DEPLOY_KEY, otherwise the ambient git credentials.
set -euo pipefail
SITE=$(cd "$1" && pwd); MODE=${2:-}
REPO=${PUBLIC_REPO:-lelelilo-studios/Finakids}
if [ -n "${SITE_DEPLOY_KEY:-}" ]; then
  mkdir -p ~/.ssh && echo "$SITE_DEPLOY_KEY" > ~/.ssh/site_key && chmod 600 ~/.ssh/site_key
  ssh-keyscan github.com >> ~/.ssh/known_hosts 2>/dev/null
  export GIT_SSH_COMMAND="ssh -i ~/.ssh/site_key -o IdentitiesOnly=yes"
fi
URL="git@github.com:$REPO.git"
WORK=$(mktemp -d)
git config --global user.name "${GIT_AUTHOR_NAME:-github-actions[bot]}" >/dev/null 2>&1 || true
git config --global user.email "${GIT_AUTHOR_EMAIL:-41898282+github-actions[bot]@users.noreply.github.com}" >/dev/null 2>&1 || true
if [ "$MODE" = "--keep-downloads" ] && git clone --depth 1 "$URL" "$WORK/current" 2>/dev/null; then
  if [ -d "$WORK/current/downloads" ]; then cp -r "$WORK/current/downloads" "$SITE/"; fi
  if [ -f "$WORK/current/README.md" ] && [ ! -f "$SITE/README.md" ]; then cp "$WORK/current/README.md" "$SITE/"; fi
fi
cd "$SITE"
rm -rf .git
git init -q -b main
git add -A
git commit -q -m "Publicar Finakids $(date -u +%Y-%m-%dT%H:%MZ)"
git push -f "$URL" main
echo "Published to https://github.com/$REPO"

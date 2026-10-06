#!/usr/bin/env bash
# Phase 0 / Step 1: check that the Steam Deck host is ready for huectl work.
# Read-only: it inspects the system and changes nothing. Safe to re-run.
#
# Usage: tools/prep-deck.sh
# Exit code is the number of failed checks (0 = ready).

set -u

repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
fails=0

pass() { printf '  \e[32mPASS\e[0m %s\n' "$1"; }
fail() { printf '  \e[31mFAIL\e[0m %s\n' "$1"; fails=$((fails + 1)); }
warn() { printf '  \e[33mWARN\e[0m %s\n' "$1"; }

echo "Virtualization"
# /dev/kvm is the real test: lscpu doesn't always print a Virtualization line.
if [ -c /dev/kvm ] && [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
    pass "/dev/kvm exists and is read/writable by $(id -un)"
else
    fail "/dev/kvm missing or not accessible (enable SVM in BIOS? kvm group?)"
fi

echo "Storage and memory"
free_gb=$(df -BG --output=avail "$repo_dir" | tail -1 | tr -dc '0-9')
if [ "$free_gb" -ge 30 ]; then
    pass "${free_gb} GB free at $repo_dir (need 30+ for the VM)"
else
    fail "only ${free_gb} GB free at $repo_dir (need 30+)"
fi
mem_gb=$(awk '/MemTotal/ {print int($2/1024/1024)}' /proc/meminfo)
if [ "$mem_gb" -ge 8 ]; then
    pass "${mem_gb} GB RAM (VM wants 4)"
else
    warn "${mem_gb} GB RAM; a 4 GB VM may be tight"
fi

echo "Container tooling"
for tool in distrobox podman; do
    if command -v "$tool" >/dev/null; then
        pass "$tool found ($("$tool" --version 2>&1 | head -1))"
    else
        fail "$tool not found"
    fi
done

echo "Sudo (needed only for things done with your explicit OK)"
if passwd -S "$(id -un)" 2>/dev/null | awk '{exit ($2 == "P") ? 0 : 1}'; then
    pass "password is set for $(id -un)"
else
    warn "could not confirm a password is set (run 'passwd' if sudo fails)"
fi

echo "Git"
if [ -n "$(git config user.name)" ] && [ -n "$(git config user.email)" ]; then
    pass "git identity: $(git config user.name) <$(git config user.email)>"
else
    fail "git user.name / user.email not set"
fi

echo
if [ "$fails" -eq 0 ]; then
    echo "Host is ready."
else
    echo "$fails check(s) failed."
fi
exit "$fails"

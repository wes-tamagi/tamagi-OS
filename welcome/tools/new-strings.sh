#!/usr/bin/bash
# Lists the English strings of tamagi-welcome that have no translation yet:
# every string the interface looks up, minus the keys already in
# system_files/usr/share/tamagi/i18n/pt.tsv, catalog.tsv and flatpaks.list.
#   tools/new-strings.sh > NEW_STRINGS.txt
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
share=${SHARE:-$here/../system_files/usr/share/tamagi}
{
    # tr("…") and the translation keys held in tables.
    grep -ohP '\btr\(&?"\K([^"\\]|\\.)*(?=")' "$here"/src/app.rs "$here"/src/askpass.rs "$here"/src/ui.rs "$here"/src/setup_app.rs
    # The installer: page names and the user name checks.
    sed -n '/fn label(self)/,/^    }/p' "$here"/src/setup_app.rs | grep -oP '=> "\K[^"]*'
    grep -oP 'Err\("\K[^"]*' "$here"/src/setup.rs
    # Badge labels and explanations (method.rs), page names (flow.rs), step
    # titles and browser methods (plan.rs), profile cards and tips (app.rs).
    sed -n '/pub fn label/,/^    }/p; /pub fn explanation/,/^    }/p; /pub fn short/,/^    }/p' "$here"/src/method.rs "$here"/src/flow.rs | grep -oP '=> "\K[^"]*'
    grep -oP 'title: "\K[^"]*' "$here"/src/plan.rs
    sed -n '/^pub const BROWSER_METHODS/,/^];/p' "$here"/src/plan.rs | grep -oP '", "\K[^"]*'
    sed -n '/^const PROFILE_KEYS/,/^];/p; /for tip in \[/,/\] {/p' "$here"/src/app.rs | grep -oP '^\s*"\K[^"]*(?=",$)'
} | sort -u | grep -vxFf <(cut -f1 "$share/i18n/pt.tsv") || true

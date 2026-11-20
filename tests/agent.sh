#!/usr/bin/bash
# Suíte de evals de ponta a ponta da IA local, DENTRO do contêiner da imagem:
# baixa o preset escolhido (padrão: mini, 2,8 GB), sobe o llama-server em modo
# roteador e executa 5 evals (tool calling, política imutável, tamagi_action,
# resposta em inglês e diagnóstico de falha de boot).
#
#   just test-agent
#   TAMAGI_EVAL_PRESET=agente just test-agent   # benchmark com Ornith 1.5 9B
set -uo pipefail

PRESET="${TAMAGI_EVAL_PRESET:-mini}"
export HOME=/tmp/agent-home TAMAGI_AI_MODELS=/models
mkdir -p "$HOME"

pass=0; fail=0
eval_ok()  { echo "  [PASS] $*"; pass=$((pass + 1)); }
eval_bad() { echo "  [FAIL] $*"; fail=$((fail + 1)); }

echo "== tamagi-ai pull $PRESET"
model=$(tamagi-ai pull "$PRESET" | tail -1)
tamagi-ai use "$model"
tamagi-ai list

echo "== opencode enxerga o provedor local"
jq -c '(.provider.tamagi | {baseURL: .options.baseURL, models: (.models | keys)}), {model}' \
    "$HOME/.config/opencode/opencode.json"
opencode models 2>/dev/null | grep '^tamagi/' || echo "(opencode models não listou o provedor local)"

echo "== subindo o llama-server (modo roteador com --jinja e --fit on)"
tamagi-ai serve > /tmp/llama.log 2>&1 &
server=$!
trap 'kill $server 2>/dev/null || true' EXIT
for _ in $(seq 1 60); do
    curl -fsS -o /dev/null http://127.0.0.1:9931/health 2>/dev/null && break
    sleep 0.5
done
echo "modelos expostos pela API: $(curl -fsS http://127.0.0.1:9931/v1/models | jq -c '[.data[].id]')"
echo "interface web: HTTP $(curl -s --compressed -o /tmp/ui.html -w '%{http_code}' http://127.0.0.1:9931/), $(grep -ci '<html' /tmp/ui.html) documento HTML"

echo
echo "== Eval 1/5: ferramenta shell/read_file (/usr/lib/os-release)"
out1=$(tamagi-agent -y "Qual é o PRETTY_NAME deste sistema? Leia /usr/lib/os-release para descobrir." 2>&1) || true
echo "$out1"
if grep -qE '^\S*(\$ |ler )' <<<"$out1" && grep -q 'Tamagi' <<<"$out1"; then
    eval_ok "Eval 1: tool calling leu os-release e retornou Tamagi"
else
    eval_bad "Eval 1: não usou ferramenta ou não retornou Tamagi"
fi

echo
echo "== Eval 2/5: ação estruturada tamagi_action (ai.status)"
out2=$(tamagi-agent -y "Use a ferramenta tamagi_action com action_id ai.status e diga qual porta o servidor local usa." 2>&1) || true
echo "$out2"
if grep -q '\[ai.status\]' <<<"$out2" && grep -q '9931' <<<"$out2"; then
    eval_ok "Eval 2: usou tamagi_action [ai.status] e identificou porta 9931"
else
    eval_bad "Eval 2: não chamou tamagi_action [ai.status] corretamente"
fi

echo
echo "== Eval 3/5: política de sistema imutável (recusar dnf install)"
out3=$(tamagi-agent "Como instalo um pacote de sistema ou app gráfico no Tamagi? Responda em 2 frases sem rodar comandos." 2>&1) || true
echo "$out3"
if grep -qiE 'flatpak|tamagi-layer|distrobox|mise' <<<"$out3" && ! grep -qiE 'rode `dnf install`|use `sudo dnf install`' <<<"$out3"; then
    eval_ok "Eval 3: respeitou o modelo imutável (Flatpak/distrobox/mise/tamagi-layer)"
else
    eval_bad "Eval 3: não citou os mecanismos corretos do Tamagi"
fi

echo
echo "== Eval 4/5: resposta em inglês (LANG=en_US.UTF-8)"
out4=$(LANG=en_US.UTF-8 TAMAGI_LANG=en tamagi-agent "In one short sentence, what is Tamagi?" 2>&1) || true
echo "$out4"
if grep -qiE '\b(the|is|atomic|system|linux|immutable)\b' <<<"$out4"; then
    eval_ok "Eval 4: respondeu em inglês quando LANG=en_US.UTF-8"
else
    eval_bad "Eval 4: resposta não parece estar em inglês"
fi

echo
echo "== Eval 5/5: diagnóstico de falha de boot (greenboot)"
fake_report=$'Data: 20/11/2026 08:00\nDigest em quarentena: sha256:deadbeef\n== Verificações de saúde ==\nFALHOU: cosmic-greeter.service inactive\n== Serviços que falharam ==\ncosmic-greeter.service\n'
out5=$(LANG=en_US.UTF-8 TAMAGI_LANG=en tamagi-agent "A system update failed health check and rolled back. Explain simply in 3 sentences what failed. Do not run commands." <<<"$fake_report" 2>&1) || true
echo "$out5"
if grep -qi 'cosmic-greeter' <<<"$out5" && ! grep -qE '^\$ ' <<<"$out5"; then
    eval_ok "Eval 5: diagnosticou cosmic-greeter sem rodar comandos"
else
    eval_bad "Eval 5: falhou no diagnóstico do relatório de boot"
fi

echo
echo "=== Scorecard ($PRESET / $model): $pass/5 passaram, $fail falharam ==="
if (( fail > 0 )); then
    tail -15 /tmp/llama.log
    exit 1
fi

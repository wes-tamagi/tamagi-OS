#!/usr/bin/bash
# Teste de ponta a ponta da IA local, DENTRO do contêiner da imagem: baixa o
# preset "mini" pelo tamagi-ai, sobe o llama-server em modo roteador e faz o
# agente resolver um pedido que exige a ferramenta shell.
#   just test-agent        (baixa 2,8 GB na primeira vez, para .cache/models)
set -euo pipefail

export HOME=/tmp/agent-home TAMAGI_AI_MODELS=/models
mkdir -p "$HOME"

echo "== tamagi-ai pull mini"
tamagi-ai pull mini
tamagi-ai use gemma-4-E2B
tamagi-ai list

echo "== opencode enxerga o provedor local"
jq -c '(.provider.tamagi | {baseURL: .options.baseURL, models: (.models | keys)}), {model}' \
    "$HOME/.config/opencode/opencode.json"
opencode models 2>/dev/null | grep '^tamagi/' || echo "(opencode models não listou o provedor local)"

echo "== subindo o llama-server (modo roteador)"
tamagi-ai serve > /tmp/llama.log 2>&1 &
server=$!
trap 'kill $server 2>/dev/null || true' EXIT
for _ in $(seq 1 60); do
    curl -fsS -o /dev/null http://127.0.0.1:9931/health 2>/dev/null && break
    sleep 0.5
done
echo "modelos expostos pela API: $(curl -fsS http://127.0.0.1:9931/v1/models | jq -c '[.data[].id]')"
# A interface é servida comprimida; como um navegador, o curl precisa aceitar gzip.
echo "interface web: HTTP $(curl -s --compressed -o /tmp/ui.html -w '%{http_code}' http://127.0.0.1:9931/), $(grep -ci '<html' /tmp/ui.html) documento HTML"

echo "== agente: pedido que exige a ferramenta shell"
out=$(tamagi-agent -y "Qual é o PRETTY_NAME deste sistema? Leia /usr/lib/os-release para descobrir." 2>&1) || true
echo "$out"
echo
if grep -qE '^\S*(\$ |ler )' <<<"$out" && grep -q 'Tamagi' <<<"$out"; then
    echo "RESULTADO: ok — o modelo usou uma ferramenta e respondeu com o dado real do sistema"
else
    echo "RESULTADO: FALHA — veja a saída acima e o fim de /tmp/llama.log:"
    tail -15 /tmp/llama.log
    exit 1
fi
